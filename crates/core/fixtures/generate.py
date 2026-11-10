import json,sys
out=sys.argv[1]
champs={103:"Ahri",64:"Lee Sin",86:"Garen",222:"Jinx",412:"Thresh",157:"Yasuo",121:"Kha'Zix",122:"Darius",51:"Caitlyn",89:"Leona"}
json.dump([{"id":k,"name":v,"squarePortraitPath":""} for k,v in champs.items()],open(f"{out}/champions.json","w"),ensure_ascii=False,indent=1)
rows=[(103,4,8,6,18000,21000,10800,171,10,19,"MIDDLE","SOLO"),(64,5,6,9,14000,29000,10500,30,140,24,"JUNGLE","NONE"),(86,3,7,5,12000,34000,9900,160,4,12,"TOP","SOLO"),(222,8,7,4,21000,17000,12500,190,8,14,"BOTTOM","CARRY"),(412,1,6,11,5000,24000,7300,28,0,48,"BOTTOM","SUPPORT"),
      (157,11,3,9,24000,22000,14300,205,9,15,"MIDDLE","SOLO"),(121,9,5,10,17000,25000,13000,25,150,27,"JUNGLE","NONE"),(122,6,4,8,16000,38000,12100,175,6,13,"TOP","SOLO"),(51,7,4,9,20000,16000,12900,201,4,18,"BOTTOM","CARRY"),(89,1,5,17,6000,31000,8000,30,0,52,"BOTTOM","SUPPORT")]
ITEMS=[2501, 2502, 2503, 2504, 2510, 2512, 2517, 2520, 2522, 2523, 2525, 3003, 3004, 3026, 3031, 3032, 3033, 3036, 3039, 3046, 3053, 3065, 3068, 3071, 3072, 3073, 3074, 3078, 3083, 3084, 3085, 3087, 3089, 3091, 3094, 3095, 3100, 3102, 3110, 3115]
def gear_for(n): return {f"item{i}":(0 if i==5 and n%3==0 else ITEMS[(n*3+i)%len(ITEMS)]) for i in range(6)}|{"item6":3340}
def stats(win,k,d,a,n=0,**x): return {"win":win,"kills":k,"deaths":d,"assists":a,"champLevel":15,**gear_for(n),"perk0":8112,"perkPrimaryStyle":8100,"perkSubStyle":8200,**x}
ps=[];ids=[]
for i,r in enumerate(rows):
    ps.append({"participantId":i+1,"teamId":100 if i<5 else 200,"championId":r[0],"spell1Id":4,"spell2Id":11 if r[10]=="JUNGLE" else 14,"timeline":{"lane":r[10],"role":r[11]},"stats":stats(i>=5,r[1],r[2],r[3],n=i,totalDamageDealtToChampions=r[4],totalDamageTaken=r[5],goldEarned=r[6],totalMinionsKilled=r[7],neutralMinionsKilled=r[8],visionScore=r[9])})
    p="demo-me" if i==0 else ("demo-ally-%d"%i if i<5 else "demo-enemy-%d"%i)
    ids.append({"participantId":i+1,"player":{"puuid":p,"gameName":"EjemploUno" if i==0 else "EjemploRival" if i>4 else "EjemploAliado","tagLine":"LAS"}})
teams=[{"teamId":100,"win":"Fail","baronKills":0,"dragonKills":1,"riftHeraldKills":1,"hordeKills":3,"towerKills":3,"inhibitorKills":0,"bans":[{"championId":-1},{"championId":122}]},{"teamId":200,"win":"Win","baronKills":1,"dragonKills":3,"riftHeraldKills":0,"hordeKills":3,"towerKills":9,"inhibitorKills":2,"bans":[{"championId":412}]}]
json.dump({"gameId":1001,"gameDuration":1812,"gameCreation":1794300000000,"queueId":420,"participants":ps,"participantIdentities":ids,"teams":teams},open(f"{out}/game.json","w"),indent=1)
# Diferencia final buscada: -7000; a los 5 minutos: +500
def frame(ms,mine,theirs):
    pf={}
    m=ms//60000
    for i in range(1,11):
        gold=(mine if i==1 else 0) if i<=5 else (theirs if i==6 else 0)
        pf[str(i)]={"participantId":i,"totalGold":gold,"minionsKilled":8*m,"jungleMinionsKilled":5 if i in (1,6) and m else 0,"level":1+m//2,"xp":400*m + (100 if i==6 else 0)}
    return {"timestamp":ms,"participantFrames":pf,"events":[]}
pts=[(0,0,0),(5,5500,5000),(10,12000,13000),(15,20000,22000),(20,27000,31000),(25,34000,39500),(30,40000,47000)]
frames=[frame(m*60000,a,b) for m,a,b in pts]
frames[1]["events"]=[{"type":"CHAMPION_KILL","timestamp":310000,"killerId":1,"victimId":6,"assistingParticipantIds":[]},{"type":"CHAMPION_KILL","timestamp":480000,"killerId":6,"victimId":1,"assistingParticipantIds":[]},{"type":"CHAMPION_KILL","timestamp":520000,"killerId":2,"victimId":7,"assistingParticipantIds":[1]},{"type":"CHAMPION_KILL","timestamp":530000,"killerId":8,"victimId":3,"assistingParticipantIds":[6]}]
frames[2]["events"]=[{"type":"ELITE_MONSTER_KILL","timestamp":640000,"killerId":7,"monsterType":"DRAGON","monsterSubType":"FIRE_DRAGON"},{"type":"ELITE_MONSTER_KILL","timestamp":650000,"killerId":0,"monsterType":"RIFTHERALD","monsterSubType":""}]
frames[3]["events"]=[{"type":"BUILDING_KILL","timestamp":900000,"killerId":1,"teamId":200,"buildingType":"TOWER_BUILDING"},{"type":"BUILDING_KILL","timestamp":960000,"killerId":0,"teamId":100,"buildingType":"TOWER_BUILDING"}]
frames[4]["events"]=[{"type":"BUILDING_KILL","timestamp":1250000,"killerId":6,"teamId":100,"buildingType":"INHIBITOR_BUILDING"}]
json.dump({"frames":frames},open(f"{out}/timeline.json","w"))
recent=[(1001,103,False,4,8,6,1812,420,"MIDDLE"),(1002,103,True,9,2,7,1620,420,"MIDDLE"),(1003,157,False,3,10,4,2055,420,"MIDDLE"),(1004,103,True,6,3,12,1490,440,"MIDDLE"),(1005,121,True,12,4,5,1733,440,"JUNGLE")]
games=[{"gameId":g,"gameDuration":t,"gameCreation":1794300000000-i*7200000,"queueId":q,"participantIdentities":[{"player":{"puuid":"demo-me"}}],"participants":[{"championId":c,"spell1Id":4,"spell2Id":11 if l=="JUNGLE" else 14,"timeline":{"lane":l,"role":"SOLO"},"stats":stats(w,k,d,a,n=i+2,goldEarned=11000,totalMinionsKilled=170,neutralMinionsKilled=8,visionScore=20,totalDamageDealtToChampions=18000)}]} for i,(g,c,w,k,d,a,t,q,l) in enumerate(recent)]
json.dump({"platformId":"DEMO","games":{"games":games}},open(f"{out}/history.json","w"),indent=1)
json.dump({"queueMap":{"RANKED_SOLO_5x5":{"tier":"GOLD","division":"II","leaguePoints":62,"wins":41,"losses":37},"RANKED_FLEX_SR":{"tier":"","division":"NA"}}},open(f"{out}/ranked.json","w"))
json.dump([{"championId":103,"championLevel":12,"championPoints":184000},{"championId":157,"championLevel":9,"championPoints":96000},{"championId":121,"championLevel":6,"championPoints":41000}],open(f"{out}/mastery.json","w"))

lp=[(1794000000000,"GOLD","III",70),(1794060000000,"GOLD","III",88),(1794130000000,"GOLD","II",6),(1794190000000,"GOLD","III",86),(1794230000000,"GOLD","II",24),(1794290000000,"GOLD","II",62)]
json.dump([{"ms":m,"queue":"SOLO","tier":t,"division":d,"lp":l} for m,t,d,l in lp],open(f"{out}/lp.json","w"))

# Misma partida en el formato de Match-V5, con compras y habilidades que el cliente local no trae.
POSITION={"MIDDLE":"MIDDLE","JUNGLE":"JUNGLE","TOP":"TOP"}
def v5_player(p,identity):
    s=p["stats"]; lane=p["timeline"]["lane"]; role=p["timeline"]["role"]
    position="UTILITY" if role=="SUPPORT" else POSITION.get(lane,"BOTTOM")
    out={"participantId":p["participantId"],"teamId":p["teamId"],"puuid":identity["player"]["puuid"],
         "riotIdGameName":identity["player"]["gameName"],"riotIdTagline":identity["player"]["tagLine"],
         "championId":p["championId"],"championName":champs[p["championId"]],"teamPosition":position,
         "summoner1Id":p["spell1Id"],"summoner2Id":p["spell2Id"],"win":s["win"],"gameEndedInEarlySurrender":False,
         "perks":{"styles":[{"style":s["perkPrimaryStyle"],"selections":[{"perk":s["perk0"]}]},{"style":s["perkSubStyle"],"selections":[]}]}}
    for k in ["champLevel","kills","deaths","assists","totalDamageDealtToChampions","goldEarned","totalMinionsKilled","neutralMinionsKilled","visionScore"]+[f"item{i}" for i in range(7)]:
        out[k]=s.get(k,0)
    return out
game=json.load(open(f"{out}/game.json"))
def v5_team(t):
    o=lambda k:{"kills":t[k]}
    return {"teamId":t["teamId"],"win":t["win"]=="Win","bans":t["bans"],"objectives":{"baron":o("baronKills"),"dragon":o("dragonKills"),"riftHerald":o("riftHeraldKills"),"horde":o("hordeKills"),"tower":o("towerKills"),"inhibitor":o("inhibitorKills")}}
info={"gameId":game["gameId"],"gameDuration":game["gameDuration"],"gameCreation":game["gameCreation"],"queueId":game["queueId"],
      "participants":[v5_player(p,i) for p,i in zip(game["participants"],game["participantIdentities"])],"teams":[v5_team(t) for t in game["teams"]]}
json.dump({"metadata":{"matchId":"DEMO_%d"%game["gameId"]},"info":info},open(f"{out}/match_v5_game.json","w"),indent=1)
timeline=json.load(open(f"{out}/timeline.json"))
extra=[(0,{"type":"ITEM_PURCHASED","participantId":1,"itemId":1056,"timestamp":15000}),(0,{"type":"ITEM_PURCHASED","participantId":1,"itemId":2003,"timestamp":16000}),
       (0,{"type":"ITEM_PURCHASED","participantId":6,"itemId":1055,"timestamp":16000}),(1,{"type":"SKILL_LEVEL_UP","participantId":1,"skillSlot":1,"levelUpType":"NORMAL","timestamp":61000}),
       (1,{"type":"SKILL_LEVEL_UP","participantId":1,"skillSlot":3,"levelUpType":"NORMAL","timestamp":200000}),(1,{"type":"SKILL_LEVEL_UP","participantId":1,"skillSlot":1,"levelUpType":"NORMAL","timestamp":260000}),
       (2,{"type":"ITEM_PURCHASED","participantId":1,"itemId":3020,"timestamp":610000}),(2,{"type":"ITEM_PURCHASED","participantId":1,"itemId":1052,"timestamp":611000}),
       (2,{"type":"ITEM_UNDO","participantId":1,"beforeId":1052,"afterId":0,"timestamp":612000}),(2,{"type":"SKILL_LEVEL_UP","participantId":1,"skillSlot":2,"levelUpType":"NORMAL","timestamp":620000}),
       (3,{"type":"SKILL_LEVEL_UP","participantId":1,"skillSlot":1,"levelUpType":"NORMAL","timestamp":905000}),(3,{"type":"ITEM_PURCHASED","participantId":1,"itemId":6655,"timestamp":930000}),
       (4,{"type":"SKILL_LEVEL_UP","participantId":1,"skillSlot":4,"levelUpType":"NORMAL","timestamp":1210000}),(4,{"type":"SKILL_LEVEL_UP","participantId":1,"skillSlot":4,"levelUpType":"EVOLVE","timestamp":1211000})]
for index,event in extra:
    timeline["frames"][index]["events"].append(event)
json.dump({"metadata":{"matchId":"DEMO_%d"%game["gameId"]},"info":{"frameInterval":60000,"frames":timeline["frames"]}},open(f"{out}/match_v5_timeline.json","w"))
