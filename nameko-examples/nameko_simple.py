from nameko.standalone.rpc import ClusterRpcClient, config
from nameko.exceptions import IncorrectSignature
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


if __name__ == "__main__":
    tempo = 4
    with rpc_proxy(CONFIG) as client:
        # test a simple messgae
        response = client.video.fibonacci(30)
        print(response)
        assert 832040 == response
        # test message with two arguments
        assert 5 == client.video.sub(b=5, a=10)
        assert 5 == client.video.sub(10, b=5)
        # test kwargs
        assert client.video.sub(10, 5) == 5
        # test arguments error
        try:
            client.video.sub(b=5, a=10, c=4)
        except IncorrectSignature as e:
            print(e)
        try:
            client.video.sub(10)
        except IncorrectSignature as e:
            print(e)
        try:
            client.video.sub(10, 5, 4)
        except IncorrectSignature as e:
            print(e)
        # make an async call
        async_response = client.video.hello.call_async("Toto")
        # make an sync call
        response = client.video.hello("Girolle")
        print(response)
        assert "Hello, Girolle!" == response
        time.sleep(tempo)
        # get the result of the aysnc call
        response_async = async_response.result()
        print(response_async)
        assert "Hello, Toto!" == response_async
        # batch a pack of async call
        start = datetime.now()
        data: list = [[i, client.video.hello.call_async(str(i))] for i in range(1000)]
        time.sleep(tempo)
        results = [[d[0], d[1].result()] for d in data]
        print(datetime.now() - start - timedelta(seconds=tempo))
