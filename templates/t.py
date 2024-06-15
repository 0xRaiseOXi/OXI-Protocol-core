import json
import time
import requests

data = {
    "password": "123",
    "id": 5294101328,
    "user_name": "Irina",
    "register_in_game": int(time.time()),
    "language": "ru"
}

headers = {
    "Content-Type": "application/json"
}

res = requests.post("https://oxiprotocol.ru/newaccount", headers=headers, data=json.dumps(data))
print(res.text)
