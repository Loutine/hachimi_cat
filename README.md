# Hachimi Cat

An open-source voice calling and conferencing software that can be "low-cost and self-hosted".

## Architecture

1. AudioService
   - depends on AudioEngine
   - Add Single Encoder binding Single/Multiple Sender Task
   - Add Multiple Decoder - Reciver Task binding Pair
2. AudioEngine
   - depends on AudioProcessing
   - Add cpal/coreaudio
3. AudioProcessing
