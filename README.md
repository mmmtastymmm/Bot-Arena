# Bot-Arena
![example event parameter](https://github.com/mmmtastymmm/Bot-Arena/actions/workflows/unit-test.yml/badge.svg?event=push)
[![codecov](https://codecov.io/gh/mmmtastymmm/Bot-Arena/branch/main/graph/badge.svg?token=R057I3M5PS)](https://codecov.io/gh/mmmtastymmm/Bot-Arena)

For letting bots compete against each other.


## Running

### Local Machine
To run on a local machine a cargo run command can be used.

```bash
cargo run --release -- --n-call-bots=21
```

### Docker
A docker image is provided. Below would be an example of running the server with 21 call bots (for testing purposes). 
It also publishes the default server port to the local machine.

```bash
docker run --rm -it -p 10100:10100 mmmtastymmm/bot-arena:main --n-call-bots=21
```

## Interface


## Coverage Sunburst Chart 
because why not

![code](https://codecov.io/gh/mmmtastymmm/Bot-Arena/branch/main/graphs/sunburst.svg?token=R057I3M5PS)