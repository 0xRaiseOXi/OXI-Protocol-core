import time
import requests
import json

headers = {
    "Content-Type": "application/json"
}
data = {
    "id": 1070221127,
    "first_name": "0xRaiseX",
    "last_name": "",
    "username": "maximusnescafe"
}
res = requests.post("https://oxiprotocol.ru/api/data", headers=headers, data=json.dumps(data))
print(res)
print(res.text)
print(res.json())

# data = {
#     "password": "123",
#     "id": 1070221127,
#     "user_name": "maximusnescafe", 
#     "register_in_game": int(time.time()),
#     "language": "ru",
# }

# res = requests.post("https://oxiprotocol.ru/newaccount", headers=headers, data=json.dumps(data))
# print(res)
# print(res.text)
# print(res.json())
