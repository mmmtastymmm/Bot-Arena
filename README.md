# Bot-Arena
![example event parameter](https://github.com/mmmtastymmm/Bot-Arena/actions/workflows/unit-test.yml/badge.svg?event=push)
[![codecov](https://codecov.io/gh/mmmtastymmm/Bot-Arena/branch/main/graph/badge.svg?token=R057I3M5PS)](https://codecov.io/gh/mmmtastymmm/Bot-Arena)

A server that registers clients and runs a Texas Hold'em game.

The server starts a websocket server at a specified port, waits for clients to connect, and then starts the game.

## Running
The server may be run locally or with docker. It also contains basic training bots that may aid in development that can be enabled at runtime via command line arguments. 

### Local Machine
To run on a local machine a cargo run command can be used. This command also includes 21 training call bots that the server will also initialize.

```bash
cargo run --release -- --n-call-bots=21
```

To install cargo follow the instructions here: https://www.rust-lang.org/tools/install

### Docker
To start the server in the same way using docker run the following command:

```bash
docker run --pull always --rm -it -p 10100:10100 mmmtastymmm/bot-arena:main --n-call-bots=21
```

## Rules

1. The client has one second from the time the server sends the game state to respond with its action.
2. Any language may be used.
3. TODO Libraries
4. No internet access will be provided so do not call any web API
5. The following resources will be provided to each bot
   1. 2 virtual CPU
   2. 3 GB of RAM
6. Hacking your fellow competitors or the server is frowned upon.
7. TODO DEADLINE
8. Each participant must submit their own bot for the compilation (there are no teams submissions)
9. The submission must be a folder with a top level dockerfile able to build an image. That image will be used to start 
a container that will be used to play the game. Below are some example repos to get you started.
   1. TODO Example Python
   2. TODO Example Rust


## API
The client and server communicate to each other over websockets using json.
### Client (Your Bot)
The client has to send 1 of 4 actions to the server to take an action.
1. Call: `{"action": "call"}`
   1. Match the current highest bet.
2. Fold: `{"action": "fold"}`
   1. Stop playing the hand
3. Raise: `{"action": "raise", "amount": 1}`
   1. Raise the current highest bet by some amount. Note the amount is an extra field required here.
4. Check: `{"action": "check"}`
   1. Effectively a bet of zero.

*Note*:

Some edge cases are discussed below:
1. An invalid message is considered a fold action.
2. Raise values are clamped between the lowest valid raise and the highest value raise.
3. If an invalid check occurs (where a call or raise is required), that check action is converted to a fold action.

### Server

The server message is a json object that contains the following fields:
- **id** (integer): 
  - Unique identifier for the current player.

- **current_bet** (integer): 
  - The player's current bet amount in the game.

- **cards** (array of strings): 
  - List of cards currently held by the player. Each card is represented by its rank and suit (e.g., "[ 6♣ ]").

- **hand_number** (integer): 
  - The number of the current hand being played.

- **current_highest_bet** (integer): 
  - The highest bet placed in the current hand.

- **flop** (array of strings): 
  - List of the three community cards dealt in the flop stage, with each card represented by its rank and suit. If not revealed yet the list just contains the string `"Hidden"`

- **turn** (string): 
  - The community card dealt in the turn stage, represented by its rank and suit. If not revealed yet it is the string `"Hidden"`.

- **river** (string): 
  - The community card dealt in the river stage, represented by its rank and suit. If not revealed yet it is the string `"Hidden"`.

- **dealer_button_index** (integer): 
  - The index (position) of the dealer in the current hand. The next player will be the first to bet.

- **players** (array of objects): 
  - List of players participating in the game. Each player object contains:
    - **id** (integer): 
      - Unique identifier for the player.
    - **player_state** (object): 
      - The state of the player, which includes:
        - **state_type** (string): 
          - The type of state (e.g., "active", "folded").
        - **details** (object): 
          - Additional details about the state, which can include "bet" (integer) for active players.
    - **total_money** (integer): 
      - The total amount of money the player currently has.

- **actions** (array of strings): 
  - List of actions taken during the current hand, each described in a string (e.g., "Player 2 took action Call.").

- **previous_actions** (array of strings): 
  - List of actions taken during the previous hand, each described in a string.

## Coverage Sunburst Chart 
Shows code coverage in a pretty way.

![code](https://codecov.io/gh/mmmtastymmm/Bot-Arena/branch/main/graphs/sunburst.svg?token=R057I3M5PS)