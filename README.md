## How order flow works
![alt text](image.png)

## Setup
1. Run redis
```
docker run -d --name redis-fire -p 6379:6379 redis:8.0.1-alpine3.21
```
2. Run service with logging
```
RUST_LOG=info cargo run
```