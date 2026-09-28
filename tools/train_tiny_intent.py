#!/usr/bin/env python3
from pathlib import Path
import struct, random

MAGIC=b'STAI1\0\0\0'
VERSION=1
CLASSES=["exact","fuzzy","related","content","cleanup","web","help","unknown"]
BUCKETS=131072
PARAMS=len(CLASSES)*BUCKETS

DATA={
"exact":[
"node.exe bul", "dosya adı node", "find notepad.exe", "exact file cargo.exe", "adı package.json olan dosya", "search filename chrome.exe", "rustc nerede", "find file readme.md", "dosya bul", "bu isimdeki dosyayı ara"],
"fuzzy":[
"adını yanlış yazmış olabilirim notpad", "benzer isimli dosyayı bul", "typo ile ara", "yaklaşık eşleşme yap", "fuzzy search nod js", "ismi buna benzeyen dosya", "yanlış yazdım bul", "close filename match"],
"related":[
"node js ile alakalı her şeyi bul", "rust ile ilgili dosyalar", "python ekosistemini göster", "npm ve node şeylerini bul", "steam ile alakalı her şey", "git dosyalarını ve ayarlarını bul", "related node js", "uygulamayla ilişkili dosyalar"],
"content":[
"içinde password yazan dosyaları bul", "dosya içeriğinde api key ara", "content search hello world", "kodların içinde websocket geçen yerler", "metin içinde node server yazan dosyalar", "logların içinde error ara", "dosyanın içinde şu kelime var", "source code içinde function ara"],
"cleanup":[
"gereksiz dosyaları bul", "neyi silebilirim", "cache temizle", "çöp dosyaları analiz et", "eski node_modules gereksiz mi", "temp dosyalarını bul", "diskte gereksiz şeyleri göster", "cleanup analysis", "duplicate ve cache bul"],
"web":[
"bu dll ne işe yarıyor internette araştır", "bilinmeyen dosyayı google da ara", "webde bu dosyayı araştır", "publisher bilinmiyor internetten bak", "bu exe güvenilir mi araştır", "lookup unknown file online"],
"help":[
"yardım", "hangi komutlar var", "help", "nasıl kullanılır", "search tool kullanımı", "komutları göster"],
"unknown":[
"merhaba", "bugün hava nasıl", "rastgele bir şey", "selam", "bilmiyorum", "test"]
}

def fnv(data: bytes)->int:
    h=0xcbf29ce484222325
    for b in data:
        h ^= b
        h = (h*0x100000001b3)&0xffffffffffffffff
    return h

def normalize(s):
    return ''.join(ch.lower() if ch.isalnum() else ' ' for ch in s)

def features(s):
    n=normalize(s)
    words=[w for w in n.split() if w]
    feats=set()
    for w in words:
        feats.add(f'w:{w}')
        if len(w)>=3:
            for i in range(len(w)-2): feats.add(f'c3:{w[i:i+3]}')
    for a,b in zip(words,words[1:]): feats.add(f'b:{a}_{b}')
    return [fnv(x.encode('utf-8'))%BUCKETS for x in feats][:96]

samples=[]
for label,phrases in DATA.items():
    y=CLASSES.index(label)
    for p in phrases: samples.append((p,y))

weights=[bytearray(BUCKETS) for _ in CLASSES]  # store signed as +128 encoding while training helper converts
bias=[0]*len(CLASSES)
def get(c,b): return weights[c][b]-128
def setv(c,b,v): weights[c][b]=max(-127,min(127,v))+128
# initialize encoded zero
for c in range(len(CLASSES)): weights[c][:]=bytes([128])*BUCKETS

def scores(text):
    fs=features(text)
    return [bias[c]+sum(get(c,b) for b in fs) for c in range(len(CLASSES))]

rnd=random.Random(1337)
for epoch in range(180):
    rnd.shuffle(samples)
    for text,y in samples:
        fs=features(text)
        sc=scores(text)
        pred=max(range(len(sc)), key=sc.__getitem__)
        if pred!=y:
            for b in fs:
                setv(y,b,get(y,b)+2)
                setv(pred,b,get(pred,b)-2)
            bias[y]+=1; bias[pred]-=1

correct=sum(max(range(len(CLASSES)), key=lambda c:scores(t)[c])==y for t,y in samples)
print(f'train={correct}/{len(samples)} params={PARAMS}')

out=Path(__file__).resolve().parents[1]/'models'/'tiny-intent-v1.stm'
with out.open('wb') as f:
    f.write(MAGIC)
    f.write(struct.pack('<HHII',VERSION,len(CLASSES),BUCKETS,PARAMS))
    f.write(struct.pack('<8i',*bias))
    f.write(bytes(12))
    for c in range(len(CLASSES)):
        # convert encoded 0..255 back to signed i8 byte representation
        f.write(bytes(((x-128)&0xff) for x in weights[c]))
print(out, out.stat().st_size)
for q in ["node js ile alakalı her şeyi bul", "içinde websocket geçen dosyalar", "gereksiz cache dosyaları", "notpad yanlış yazdım", "node.exe bul"]:
    sc=scores(q); print(q, CLASSES[max(range(len(sc)), key=sc.__getitem__)], max(sc))
