# blackedoutchat
A decentralized chat program that uses the Tor network to remain anonymous and punch through NATs.

## Architecture
TODO

## TODO
This is a project that I worked on and dropped a year ago. I'm rebuilding it from scratch (started on 23rd April 2022).

- [x] Tor process wrapper
  - [x] Starting the process in a separate thread
  - [x] Configuring Tor to use Unix sockets to not conflict with existing Tor instances
  - [x] Other Tor configuration (data directory, etc.)
  - [x] Change file permissions on data directory to satisfy Tor's requirements (`chmod 700` seems to do the trick)
  - [x] Ctrl+C and `SIGTERM` handler in parent process that will send Tor a `SIGTERM` before terminating
  - [x] Asynchronous loop to monitor the condition of Tor (i.e. if it exits, then terminate the parent process for now)
- [ ] P2P connections
  - [ ] Unix socket to listen to incoming connections from Tor
  - [ ] Interface to accept connection requests to other peers
  - [x] Global state to keep track of all connected peers
  - [ ] Authentication (to prove that the peer connecting to you is who they say they are)
    - [ ] You will send a random 256-bit token to the peer
    - [ ] Peer will send you the onion address that they claim to be and also the sign the token using their ed25519 key and send the signatture
    - [ ] You will verify the signature by deriving the ed25519 public key from the onion address
    - [x] There is no need for you to prove who you are to them as they are the ones connecting to your onion address and Tor proves it internally
- [ ] Cryptography (post-quantum hybrid approach)
  - [x] Tor already encrypts traffic with classical methods
  - [x] Post-quantum key exchange between peers
  - [ ] A post-quantum public key must be shared between peers on first connect and saved. Use this to send tokens and verify signatures
- [ ] Storage (chat messages, peer info, etc.)
  - [ ] Evaluate which method of data storage is most suitable
- [ ] Client (web based)
  - [ ] A local WebSocket server which your browser can connect to
    - [ ] Password authentication using OPAQUE
    - [ ] (TODO client features)
