import random
import json

list_rand: list[int] = [random.randint(0, 35) for _ in range(1000)]

with open("examples/data_set.json", "w") as f:
    json.dump(
        list_rand,
        f,
    )
