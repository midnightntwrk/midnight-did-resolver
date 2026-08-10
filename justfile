# Run `just` (or `just demo`) to start the stable Docker demo and wait for both
# application health endpoints.

demo:
    ./demo/run.sh up
    ./demo/run.sh health

demo-down:
    ./demo/run.sh down

demo-clean:
    ./demo/run.sh clean

demo-logs:
    ./demo/run.sh logs

demo-config:
    ./demo/run.sh config
