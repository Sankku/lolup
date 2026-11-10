//! Llamadas a Claude en streaming. La key vive solo en este proceso.

use futures_util::StreamExt;
use serde_json::{Value, json};

const MODEL: &str = "claude-opus-5-5";
const API_URL: &str = "https://api.anthropic.com/v1/messages";

pub const REVIEW_PROMPT: &str = "Sos un analista de League of Legends y escribís la revisión \
post-partida de un jugador, en español claro y directo. La ficha es tu única fuente: usá sus \
números tal como están. Si algo importante no aparece en la ficha, como posiciones, visión del \
mapa o momentos de la partida, decí que no se puede saber. No describas habilidades, valores ni \
cambios de parche. Cuando nombres un campeón, aclará si es de tu equipo o del rival. \
Escribí en texto plano, sin markdown y en menos de 180 palabras, con este orden: una línea de \
veredicto; tres claves, cada una con un dato y qué cambiar; y al final una línea que empiece con \
'Próxima partida:' con una sola meta medible.";

pub const PLAN_PROMPT: &str = "Preparás un plan fijo para que un jugador de League of Legends lo \
lea antes de empezar la partida. Usá solo los campeones de la ficha y lo que se sabe de ellos en \
general. No menciones números de habilidades ni tiempos de reutilización, ni instrucciones que \
dependan de lo que ocurra durante la partida. Escribí en español, en texto plano, exactamente \
cuatro líneas de menos de 14 palabras cada una, que empiecen con 'Objetivo:', 'Fuerte en:', \
'Cuidado con:' y 'Recordá:'.";

#[derive(Debug, PartialEq)]
enum Event {
    Text(String),
    Stop(String),
    Error(String),
}

fn parse_event(line: &str) -> Option<Event> {
    let data: Value = serde_json::from_str(line.strip_prefix("data:")?.trim()).ok()?;
    match data["type"].as_str()? {
        "content_block_delta" if data["delta"]["type"] == "text_delta" => {
            Some(Event::Text(data["delta"]["text"].as_str()?.to_string()))
        }
        "message_delta" => Some(Event::Stop(data["delta"]["stop_reason"].as_str()?.to_string())),
        "error" => Some(Event::Error(data["error"]["message"].as_str()?.to_string())),
        _ => None,
    }
}

/// Saca las líneas completas. Una letra con tilde puede llegar partida entre dos fragmentos.
fn drain_lines(buffer: &mut Vec<u8>, chunk: &[u8]) -> Vec<String> {
    buffer.extend_from_slice(chunk);
    let Some(end) = buffer.iter().rposition(|b| *b == b'\n') else {
        return Vec::new();
    };
    let complete: Vec<u8> = buffer.drain(..=end).collect();
    String::from_utf8_lossy(&complete).lines().map(str::to_string).collect()
}

fn status_message(status: u16) -> String {
    match status {
        401 | 403 => "La API key no es válida.".into(),
        429 => "Se alcanzó el límite de uso. Probá de nuevo en un rato.".into(),
        s if s >= 500 => "El servicio de IA no está disponible. Probá de nuevo.".into(),
        s => format!("El servicio de IA rechazó la solicitud ({s})."),
    }
}

pub async fn stream(
    http: &reqwest::Client,
    system: &str,
    card: &str,
    mut on_text: impl FnMut(&str),
) -> Result<String, String> {
    let key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| "Falta ANTHROPIC_API_KEY.")?;
    let body = json!({
        "model": MODEL,
        // Tope de costo: la longitud real la fija el prompt.
        "max_tokens": 8000,
        "stream": true,
        "output_config": { "effort": "medium" },
        "fallbacks": "default",
        "system": system,
        "messages": [{ "role": "user", "content": card }],
    });
    let response = http
        .post(API_URL)
        .header("x-api-key", key)
        .header("anthropic-version", "2026-10-21")
        .header("anthropic-beta", "server-side-fallback-2026-07-01")
        .json(&body)
        .send()
        .await
        .map_err(|_| "No se pudo conectar con el servicio de IA.")?;
    if !response.status().is_success() {
        return Err(status_message(response.status().as_u16()));
    }

    let mut full = String::new();
    let mut buffer = Vec::new();
    let mut chunks = response.bytes_stream();
    while let Some(chunk) = chunks.next().await {
        let chunk = chunk.map_err(|_| "Se cortó la conexión con el servicio de IA.")?;
        for line in drain_lines(&mut buffer, &chunk) {
            match parse_event(&line) {
                Some(Event::Text(text)) => {
                    on_text(&text);
                    full.push_str(&text);
                }
                Some(Event::Stop(reason)) if reason == "refusal" => {
                    return Err("El modelo no pudo responder esta solicitud.".into());
                }
                Some(Event::Stop(reason)) if reason == "max_tokens" => {
                    return Err("La respuesta quedó incompleta.".into());
                }
                Some(Event::Error(message)) => return Err(message),
                _ => {}
            }
        }
    }
    Ok(full)
}

/// Respuesta de ejemplo para el modo demo sin key. Nunca se usa con datos reales.
pub async fn demo_stream(text: &str, mut on_text: impl FnMut(&str)) -> Result<String, String> {
    for word in text.split_inclusive(' ') {
        on_text(word);
        tokio::time::sleep(std::time::Duration::from_millis(28)).await;
    }
    Ok(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stream_events() {
        assert_eq!(
            parse_event(r#"data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"Hola"}}"#),
            Some(Event::Text("Hola".into()))
        );
        assert_eq!(
            parse_event(r#"data: {"type":"message_delta","delta":{"stop_reason":"refusal"}}"#),
            Some(Event::Stop("refusal".into()))
        );
        assert_eq!(
            parse_event(r#"data: {"type":"error","error":{"message":"overloaded"}}"#),
            Some(Event::Error("overloaded".into()))
        );
        assert_eq!(parse_event("event: message_start"), None);
        assert_eq!(parse_event(r#"data: {"type":"content_block_delta","delta":{"type":"thinking_delta"}}"#), None);
        assert_eq!(parse_event("data: no-json"), None);
    }

    #[test]
    fn keeps_accents_split_between_chunks() {
        let bytes = "data: niñó\ndata: fin".as_bytes();
        let split = 8; // en medio de la "ñ"
        let mut buffer = Vec::new();
        assert!(drain_lines(&mut buffer, &bytes[..split]).is_empty());
        assert_eq!(drain_lines(&mut buffer, &bytes[split..]), ["data: niñó"]);
        assert_eq!(drain_lines(&mut buffer, b"\n"), ["data: fin"]);
    }

    #[test]
    fn explains_http_errors() {
        assert!(status_message(401).contains("API key"));
        assert!(status_message(429).contains("límite"));
        assert!(status_message(503).contains("disponible"));
        assert!(status_message(400).contains("400"));
    }
}
