# blackedoutchat
A decentralized chat program that uses the Tor network to remain anonymous and punch through NATs.

## Architecture
TODO

## TODO
This is a project that I worked on and dropped a year ago. I'm rebuilding it from scratch (started on 23rd April 2022).

UPDATE 2023: Messages can now be sent between clients but there is no mechanism to store messages yet.

- [x] Tor process wrapper
  - [x] Starting the process in a separate thread
  - [x] Configuring Tor to use Unix sockets to not conflict with existing Tor instances
  - [x] Other Tor configuration (data directory, etc.)
  - [x] Change file permissions on data directory to satisfy Tor's requirements (`chmod 700` seems to do the trick)
  - [x] Ctrl+C and `SIGTERM` handler in parent process that will send Tor a `SIGTERM` before terminating
  - [x] Asynchronous loop to monitor the condition of Tor (i.e. if it exits, then terminate the parent process for now)
- [x] P2P connections
  - [x] Unix socket to listen to incoming connections from Tor
  - [x] Interface to accept connection requests to other peers
  - [x] Global state to keep track of all connected peers
  - [x] Authentication (to prove that the peer connecting to you is who they say they are)
    - [x] You will send a random 256-bit token to the peer
    - [x] Peer will send you the onion address that they claim to be and also the sign the token using their ed25519 key and send the signatture
    - [x] You will verify the signature by deriving the ed25519 public key from the onion address
    - [x] There is no need for you to prove who you are to them as they are the ones connecting to your onion address and Tor proves it internally
- [x] Cryptography (post-quantum hybrid approach)
  - [x] Tor already encrypts traffic with classical methods
  - [x] Post-quantum key exchange between peers
  - [ ] (Feature for later) A post-quantum public key must be shared between peers on first connect and saved. Use this to send tokens and verify signatures
- [ ] Storage (chat messages, peer info, etc.)
  - [x] Evaluate which method of data storage is most suitable (diesel with sqlite and potentially other backends later)
  - [x] A message handler that passes messages to the data storage as well as all currently connected clients
  - [ ] Actually store the messages
- [ ] Client (web based)
  - [x] Local webapp UI
  - [x] A local WebSocket server which your browser can connect to
  - [ ] Password authentication using OPAQUE
  - [ ] (TODO client features)
