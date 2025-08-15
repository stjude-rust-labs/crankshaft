## 🚀 Quick Start

### Prerequisites

- Rust 1.70+
- Protocol Buffers compiler
- Docker (optional, for container examples)

### Run Your First Example

```bash
# Simple local execution (openssl, protobuf as only dependencies outside of rust)
cargo run --bin local_example
---

### 🏠 [`local_example`](src/local_example/local_example.rs) - Local Task Execution

**Purpose**: Demonstrates core Crankshaft concepts without external dependencies.

**Key Features**:
- Task definition and execution lifecycle
- Comprehensive error handling patterns
- Performance monitoring and timing
- Sequential execution (educational comparison)

**Use Case**: Understanding Crankshaft fundamentals, onboarding new developers.

```bash
cargo run --bin local_example
```

**Expected Output**: 5 tasks execute sequentially with timing information and summary statistics.

---
