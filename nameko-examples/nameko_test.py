from nameko.standalone.rpc import ClusterRpcClient, config
import os
from datetime import datetime

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


def send_messages(name: str, count: int = 1) -> list:
    """
    send_simple_message send a message to the queue

    :param name: name of the person
    :type name: str
    """
    data = list()
    with rpc_proxy(CONFIG) as rpc:
        for i in range(count):
            i_str = str(i).zfill(4)
            data.append(rpc.video.hello(f"{name}{i_str}"))
    return data

def send_simple_message(name: str) -> str:
    """
    send_simple_message send a message to the queue

    :param name: name of the person
    :type name: str
    """
    with rpc_proxy(CONFIG) as rpc:
        return rpc.video.hello(name)


def fibonacci(n: int = 10) -> list[int]:
    """
    fibonacci send a message to the queue
    """
    data = list()
    with rpc_proxy(CONFIG) as rpc:
        for i in range(n):
            data.append(rpc.video.fibonacci(i))
    return data


if __name__ == "__main__":
    start = datetime.now()
    response = send_simple_message(True)
    print(response, datetime.now() - start)
