from nameko.standalone.rpc import ClusterRpcClient, config
import os

CONFIG = {
    "AMQP_URI": "pyamqp://{}:{}@{}/".format(
        os.getenv("RABBITMQ_USER"),
        os.getenv("RABBITMQ_PASSWORD"),
        os.getenv("RABBITMQ_HOST")
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

def send_simple_message(name):
    """
    send_simple_message send a message to the queue

    :param name: name of the person
    :type name: str
    """
    with rpc_proxy(CONFIG) as rpc:
        return rpc.video.hello(name)

if __name__ == "__main__":
    response = send_simple_message({'name': '/tmp/lolcat1.avi', 'size': 1301013})
    print(response)