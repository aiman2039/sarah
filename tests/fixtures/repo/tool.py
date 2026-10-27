"""A small Python module."""


def greet(name):
    # Say hello.
    return f"hello {name}"


class Greeter:
    def run(self):
        return greet("world")
