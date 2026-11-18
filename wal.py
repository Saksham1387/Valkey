import os
import json 

class WAL:
    def __init__(self,filename):
        self.filename = filename
        self.file = open(filename,"a", encoding="utf-8")

    def append(self,operation):
        entry = json.dumps(operation) + "\n"
        self.file.write(entry)
        self.file.flush()
        os.fsync(self.file.fileno())

    def replay(self):
        operations = []
        try:
            with open(self.filename, "r", encoding="utf-8") as f:
                for line in f:
                    try:
                        operations.append(json.loads(line))
                    except json.JSONDecodeError:
                        break
        except FileNotFoundError:
            pass
        return operations

    def close(self):
        self.file.close()