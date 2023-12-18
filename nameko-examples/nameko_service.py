from nameko.rpc import rpc


class GreetingService:
    name = "video"

    @rpc
    def hello(self, name):
        return "Hello, {}!, by nameko".format(name)
