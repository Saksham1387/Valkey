from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
import json 
import threading
from wal import WAL

app = FastAPI()

store: dict[str,str] = {}

class KeyIn(BaseModel):
    value: str

wal = WAL("wal.log")
lock = threading.Lock()

seq = 0

for op in wal.replay():
    seq = max(seq, op["seq"])
    if op["op"] == "put":
        store[op["key"]] = op["value"]
    elif op["op"] == "delete":
        store.pop(op["key"], None)

@app.put("/keys/{key}")
def put(key: str, body: KeyIn):
    global seq
    with lock:
        seq += 1
        data = {"seq":seq,"op":"put","key":key,"value":body.value}
        wal.append(data)
        store[key] = body.value
    return {"key": key, "value": body.value}

@app.get("/keys/{key}")
def get(key: str):
    if key not in store:
        raise HTTPException(404, "not found")
    return {"key": key, "value": store[key]}

@app.delete("/keys/{key}")
def delete(key: str):
    global seq
    with lock:
        seq += 1
        wal.append({"seq": seq, "op": "delete", "key": key})
        store.pop(key, None)
    return {"deleted": key}