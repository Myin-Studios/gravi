<div align="center">
  <h1>Gravi</h1>
  <p><strong>The official compiler for the Aion programming language</strong></p>

  ![Build](https://github.com/Myin-Studios/gravi/actions/workflows/release.yml/badge.svg)
  ![Version](https://img.shields.io/badge/version-0.1.5-blue)
  ![License](https://img.shields.io/badge/license-MPL--2.0-green)
</div>

---

**Aion** is a statically typed, compiled programming language designed for clarity and performance.
Its compiler, Gravi, is written in Rust and targets native code via C.

```aion
fun greet(name: string)
{
    show(fmt("Hello, {}!", name));
}

greet("world");
```

---

## Installation

### Windows
Download the latest `.msi` installer from the [Releases](https://github.com/Myin-Studios/gravi/releases) page.
The installer automatically adds `gravi` to your `PATH`.

### Linux
```bash
cargo install --git https://github.com/Myin-Studios/gravi
```

### Build from source
```bash
git clone https://github.com/Myin-Studios/gravi
cd gravi
cargo build --release
```
The binary will be at `target/release/gravi`.

---

## Usage

```bash
gravi file.nn          # compile
gravi file.nn --r      # compile and run
```

Aion source files use the `.nn` extension.

---

### Variables

```aion
var x = 42;             // inferred type
var y: f32 = 3.14;      // explicit type
mut var z: u8 = 0;      // mutable
```

### Functions

```aion
fun add(a: i32, b: i32): i32
{
    ret a + b;
}
```

### Control flow

```aion
// for-loop
loop i in 0:10
{
    show(i);
}

// while-loop
mut var i: u8 = 0;
loop i < 10
{
    i = i + 1;
}

// if-else
if x > 0
{
    show("positive");
}
else
{
    show("non-positive");
}
```

### Types

| Aion     | Description              |
|----------|--------------------------|
| `u8`–`u64` | Unsigned integers      |
| `i8`–`i64` | Signed integers        |
| `f16`–`f64` | Floating point        |
| `usize`  | Pointer-sized unsigned   |
| `bool`   | Boolean                  |
| `char`   | Single character         |
| `string` | String literal           |

### Extern functions

```aion
ext fun sqrt(x: f64): f64;
```

---

## Roadmap

- [x] Variables and type inference
- [x] Functions and recursion
- [x] Loops and conditionals
- [x] Extern function declarations
- [x] String formatting
- [ ] Enums
- [ ] Classes and interfaces (`type`)
- [ ] LLVM backend
- [ ] Generics
- [ ] Standard library

---

## Contributing

Contributions are welcome. Please open an issue before submitting a pull request for significant changes.

```bash
git clone https://github.com/Myin-Studios/gravi
cd gravi
cargo build
cargo test
```

---

## License

Distributed under the [Mozilla Public License 2.0](LICENSE).
