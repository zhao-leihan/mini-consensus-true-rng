# 🪶 Mini-Consensus + True RNG (Rust)

A minimal blockchain-style backend built with **Rust (Edition 2021)** featuring:

- **Deterministic 2-phase Consensus** — simplified leader-driven finality model.
- **True Random Number Generator (TRNG)** — collects real entropy from OS and timing jitter.
- **REST API** built with `axum` to expose endpoints for consensus and randomness.
- **Unit tests** for both modules ensuring quorum-based finality and randomness health.

---

## Features

### Mini-Consensus
- Leader-based, 2-phase model (Precommit → Commit)
- Deterministic finality when ≥ 2/3 quorum in both phases
- Linear chain with parent pointers
- Unit test: 4 validators (1 faulty) → still achieves finality

### True RNG (TRNG)
- Combines **OS `getrandom`** entropy + **timing jitter** source
- Hash conditioning with `blake3`
- `reseed()` API for new entropy collection
- Health metrics:
  - Monobit frequency test  
  - Runs test  
  - Shannon entropy estimation  
- Negative control: disabling jitter reduces entropy → demonstrates true randomness

---

## REST API

| Method | Endpoint | Description |
|--------|-----------|-------------|
| `POST` | `/propose` | Submit new block proposal |
| `GET` | `/finalized` | Get latest finalized block |
| `GET` | `/rng?len=32` | Get random bytes |
| `GET` | `/health` | Show TRNG health metrics |

Example:
```bash
curl http://localhost:8080/health
# {"healthy":true,"metrics":{"monobit_deviation":0.0019,"runs_deviation":0.0032,"shannon_entropy":7.97}}

curl http://localhost:8080/rng?len=32
# {"random_bytes":"c6de9162ad0e9897f5e2d6a862f4272f88b1ae7f49d0335c0f67e2ed536cc48b"}
```

## Build & Run
```bash
cargo build --release
cargo run --bin node
```

Then open:
```bash
http://localhost:8080/health

http://localhost:8080/rng?len=32
```
Run tests:
```bash
cargo test
```


## Sample Output
### Running Node
![Running Node](https://raw.githubusercontent.com/zhao-leihan/mini-consensus-true-rng/main/assets/running%20node.png)

### Running Tests
![Running Tests](https://raw.githubusercontent.com/zhao-leihan/mini-consensus-true-rng/main/assets/running%20test.png)

### cURL Testing
![cURL Testing](https://raw.githubusercontent.com/zhao-leihan/mini-consensus-true-rng/main/assets/curl%20testing.png)

### TRNG Health Check
![TRNG Health Check](https://raw.githubusercontent.com/zhao-leihan/mini-consensus-true-rng/main/assets/node%20healt%20check.png)

## Video Demo:

[![Watch the video](https://img.youtube.com/vi/BBES7imtXDQ/0.jpg)](https://www.youtube.com/watch?v=BBES7imtXDQ)

