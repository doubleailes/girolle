from nameko.rpc import rpc, RpcProxy
import time


class GreetingService:
    name = "video"

    video_proxy = RpcProxy("video")

    @rpc
    def hello(self, name):
        return "Hello, {}!".format(name)

    @rpc
    def double_hello(self, name):
        n_name = self.video_proxy.hello(name)
        return n_name + " " + n_name

    @rpc
    def fibonacci(self, n):
        if n <= 1:
            return n
        else:
            return self.fibonacci(n - 1) + self.fibonacci(n - 2)

    @rpc
    def sleep(self, n):
        time.sleep(n)
        return "Slept for {} seconds".format(n)
