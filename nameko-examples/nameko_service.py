from nameko.rpc import rpc


class GreetingService:
    name = "video"

    @rpc
    def hello(self, name):
        return "Hello, {}!, by nameko".format(name)

    @rpc
    def fibonacci(self, n):
        if n <= 1:
            return n
        else:
            return self.fibonacci(n - 1) + self.fibonacci(n - 2)
