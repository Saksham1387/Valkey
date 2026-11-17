from fastapi import FastAPI
from pydantic import BaseModel
import json 

app = FastAPI()

seq

store: dict[str,str] = {}

class KeyIn(BaseModel):
    value: str

@app.put("/keys/{key}")
def put(key: str, body: KeyIn):
    seq = seq + 1
    data = {"seq":seq,"op":"put","key":key,"value":body.value}
    
    with open("WAL.json", "w") as file:
        json.dump(data, file, indent=4)

    store[key] = body.value
    return {"key": key, "value": body.value}

@app.get("/keys/{key}")
def get(key: str):
    if key not in store:
        raise HTTPException(404, "not found")
    return {"key": key, "value": store[key]}

@app.delete("/keys/{key}")
def delete(key: str):
    store.pop(key, None)
    return {"deleted": key}