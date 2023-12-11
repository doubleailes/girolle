from kombu import Connection, Exchange, Queue
import os

media_exchange = Exchange('media', 'direct', durable=True)
video_queue = Queue('video', exchange=media_exchange, routing_key='video')

with Connection("amqp://{}:{}@{}/".format(
        os.getenv("RABBITMQ_USER"),
        os.getenv("RABBITMQ_PASSWORD"),
        os.getenv("RABBITMQ_HOST")
    )) as conn:

    # produce
    producer = conn.Producer(serializer='json')
    producer.publish({'name': '/tmp/lolcat1.avi', 'size': 1301013},
                      exchange=media_exchange, routing_key='video',
                      declare=[video_queue])

    # the declare above, makes sure the video queue is declared
    # so that the messages can be delivered.
    # It's a best practice in Kombu to have both publishers and
    # consumers declare the queue. You can also declare the
    # queue manually using:
    #     video_queue(conn).declare()
