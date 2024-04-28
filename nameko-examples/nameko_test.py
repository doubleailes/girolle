from nameko.standalone.rpc import ClusterRpcClient, config
from nameko.exceptions import RemoteError
import os
from datetime import datetime
import time

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

def send_message_async(name: str, count: int = 1) -> list:
    """
    send_simple_message send a message to the queue

    :param name: name of the person
    :type name: str
    """
    data = list()
    with rpc_proxy(CONFIG) as rpc:
        for i in range(count):
            i_str = str(i).zfill(4)
            data.append(rpc.video.hello.call_async(f"{name}{i_str}"))
        time.sleep(1)
        return [d.result() for d in data]

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

def test_sleep(n: int = 10) -> str:
    """
    fibonacci send a message to the queue
    """
    with rpc_proxy(CONFIG) as rpc:
        return rpc.video.sleep.call_async(n)

if __name__ == "__main__":
    start = datetime.now()
    response = send_simple_message("John Doe")
    print(response, datetime.now() - start)
    print("start async")
    for i in range(100):
        test_sleep(10)
    print("stop async")
    start = datetime.now()
    response = send_messages("John Doe", 10)
    print(response, datetime.now() - start)
    start = datetime.now()
    response = fibonacci(10)
    print(response, datetime.now() - start)
    try:
        response = send_simple_message(True)
    except RemoteError as e:
        print(e)
