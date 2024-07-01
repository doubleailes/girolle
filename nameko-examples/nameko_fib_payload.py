from nameko.standalone.rpc import ClusterRpcClient, config
from nameko.exceptions import IncorrectSignature
import os
from datetime import datetime, timedelta
import time
import json

CONFIG = {
    "AMQP_URI": "pyamqp://{}:{}@{}/".format(
        os.getenv("RABBITMQ_USER"),
        os.getenv("RABBITMQ_PASSWORD"),
        os.getenv("RABBITMQ_HOST"),
    )
}


def rpc_proxy(CONFIG) -> ClusterRpcClient:
    """
    rpc_proxy load configuration paramter et init the setup

    :param CONFIG: configuration param as a string
    :type CONFIG: str
    :return:
    :rtype: ClusterRpcClient
    """
    config.setup(CONFIG)
    return ClusterRpcClient(CONFIG)


if __name__ == "__main__":
    json_data: list[int] = json.load(open("../examples/data_set.json"))
    with rpc_proxy(CONFIG) as client:
        start = datetime.now()
        data: list = [[i, client.video.fibonacci.call_async(i)] for i in json_data]
        results: list[list[int, int]] = [[d[0], d[1].result()] for d in data]
        print(datetime.now() - start)
