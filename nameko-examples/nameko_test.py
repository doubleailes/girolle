from nameko.standalone.rpc import ClusterRpcClient, config
from nameko.exceptions import RemoteError
import os
from datetime import datetime, timedelta
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


def send_simple_message(name: str) -> str:
    """
    send_simple_message send a message to the queue

    :param name: name of the person
    :type name: str
    """
    with rpc_proxy(CONFIG) as client:
        return client.video.hello(name)
    return data


if __name__ == "__main__":
    tempo = 4
    with rpc_proxy(CONFIG) as client:
        response = client.video.fibonacci(30)
        print(response)
        assert 832040 == response
        response = client.video.sub(10, 5)
        assert 5 == response
        async_response = client.video.hello.call_async("Toto")
        response = client.video.hello("Girolle")
        print(response)
        assert "Hello, Girolle!" == response
        time.sleep(tempo)
        r = async_response.result()
        print(r)
        assert "Hello, Toto!" == r
        start = datetime.now()
        data: list = [[i, client.video.hello.call_async(str(i))] for i in range(1000)]
        time.sleep(tempo)
        results = [[d[0], d[1].result()] for d in data]
        print(datetime.now() - start - timedelta(seconds=tempo))
