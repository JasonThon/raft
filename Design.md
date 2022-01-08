# Design

## Code Structure
```
Loop Select:
    Async Task 1: 
    +----------+        +---------------+        +-----------------+
    | Listener |------->| Mesage Sender |------->| Message Channel |
    +----------+        +---------------+        +-----------------+

    Async Task 2: 
    +------------------+  poll message  +-----------------+          +-----------------------+
    | Message Receiver |<---------------| Message Handler |--------->| Replica State Machine | 
    +------------------+                +--------+--------+          +-----------------------+
                                                 |
                                                 |
                                                 |  
                                                 |
                                                 v
                                        +-------------------+         +--------------------+
                                        | Inner Resp Sender |-------->| Inner Resp Channel |
                                        +-------------------+         +--------------------+
    
    Async Task 3: 
    +------------------+  poll message +--------------------+
    | Inner Resp Recv  |<--------------| Broadcaster/Sender |----------> Tcp Stream
    +------------------+               +--------------------+
    
    Async Task 4:
    +--------+       +-----------------------+
    | Ticker |------>| Replica State Machine |
    +--------+       +-----------------------+
```

