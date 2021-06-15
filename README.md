# UDP transfer [![Build Status](https://travis-ci.com/PatrikValkovic/UDP_transfer.svg?token=ppQLqQBxth8AKnLEKLkS&branch=master)](https://travis-ci.com/PatrikValkovic/UDP_transfer)

Implementation of reliable data transfer.

## Binaries

- Sender sends data to the receiver.
```text
Usage:
  sender [OPTIONS]


Optional arguments:
  -h,--help             Show this help message and exit
  -v,--verbose          Verbose output
  --bind BIND           Address to bind to in format IP:port
  -f,--file FILE        File to send
  --packet PACKET       Maximum packet size
  --addr ADDR           Address where send data in format IP:port
  -w,--window WINDOW    Size of the window
  -t,--timeout TIMEOUT  Timeout after which resend the data
  -r,--repetition REPETITION
                        Maximum number of timeouts per packet
  -s,--sum_size SUM_SIZE
                        Size of the checksum
```
- Receiver gets the data and store them in specified directory.
```text
Usage:
  receiver [OPTIONS]


Optional arguments:
  -h,--help             Show this help message and exit
  -v,--verbose          Verbose output
  --addr ADDR           Address to bind to in format IP:port
  -d,--directory DIRECTORY
                        Directory where to store received files
  --packet PACKET       Maximum packet size
  -w,--window WINDOW    Maximum size of the window
  -t,--timeout TIMEOUT  Timeout after which resend the acknowledge packet
  -s,--checksum CHECKSUM
                        Minimum size of checksum
```
- Broker split the connection between sender and receiver. 
  It may drop some packets, randomly modify them or delay them on the way.
```text
Usage:
  broker [OPTIONS]


Optional arguments:
  -h,--help             Show this help message and exit
  -v,--verbose          Verbose output
  --sender_bind SENDER_BIND
                        Address to bind from the sender perspective in format
                        IP:port
  --receiver_bind RECEIVER_BIND
                        Address to bind from the receiver perspective in format
                        IP:port
  --sender_addr SENDER_ADDR
                        Address of the sender in format IP:port
  --receiver_addr RECEIVER_ADDR
                        Address of the receiver in format IP:port
  --packet PACKET       Maximum packet size
  -m,--delay_mean DELAY_MEAN
                        Mean value of delay
  -s,--delay_std DELAY_STD
                        Standard deviation of delay
  -d,--drop_rate DROP_RATE
                        Percentage of dropout of packets between 0 and 1
  -m,--modify MODIFY    Probability of byte modification
```

By default, receiver binds to address `127.0.0.1:3003` and store received files into `received` directory.
It accepts packets with maximum of 1500 bytes and timeouts after 5 seconds.
Window size is set to 15 packets and it needs checksum of at least 16 bytes.

Sender binds to address `127.0.0.1:3000` and sends data to address `127.0.0.1:3001`.
It sends file `input.txt` with packets of size 1500 bytes and checksum size 64 bytes.
The window size is set to 15 packets and it does at most 20 retries.

Broker binds to addresses `127.0.0.1:3001` and `127.0.0.1:3002`, so it resends data from sender to the receiver and vice versa.
It does not modify the packet in any way.

## How it works

The implementation into some extent simulates working of TCP connection using UDP packets.

1. Sender sends the `INIT` packet with properties of the connection (packet size, window size, checksum size).
1. The receiver asnwers with `INIT` packet with confirmed properties and connection identificator. 
   The properties may be different - receiver may adjust received parameters.
   If the receiver does not get the whole packet (it was trunkated on the way), it informs sender and ask it to retry.
1. The sender that starts sending data in `DATA` packet.
   Data packet consists of connection id, sequential (seq) and acknowledge (ack) number, data, and the checksum.
   The "protocol" uses fixed size sliding window [sliding window protocol](https://en.wikipedia.org/wiki/Sliding_window_protocol).
   Sequential number (position of the packet with respect to other packets) set up sender, whereas acknowledge number (last packet it received from the beginning of the file) set up receiver. 
   It is possible to transfer data both ways using the same connection, but it is not implemented.
   Note that UDP has checksum build in, so it should not be necessary.
1. When receiver get `DATA` packet it moves the window (if necessary) and answer with `DATA` packet with acknowledge number (after all necessary validation).
   The acknowledge number is sequential number of the last packet it received from the beginning of the file. That is, it is missing data packet with sequential number one greater.
   At the beginning of the communication, receiver sends acknowledge number 65535 until it receives packet with sequential number 0.
1. Sender sends packets until the receiver acknowledge it received all the data available.
1. Sender then sends `END` packet to close the connection. It then waits until it receives `END` packet from receiver.
1. When receiver get `END` packet it flush content of the connection into the file and close it.
   It then sends `END` packet back to the sender.
   
If validation on either sender or receiver part fail (for example invalid connection ID), it sends `ERR` packet.
After receiving `ERR` packet the program sends it back as a confirmation and then close the connection.

The communication has timeout and if the other side does not respond in the specified time the data are resend.

--------------

The purpose of this project was just to learn Rust and as a programming exercise.
There is no guarantee about the correctness.

Author: Patrik Valkovič

2021