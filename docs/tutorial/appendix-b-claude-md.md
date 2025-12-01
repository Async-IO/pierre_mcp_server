<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Appendix B: CLAUDE.md Compliance Checklist

Quick checklist for CLAUDE.md code standards compliance.

## Error Handling (Zero Tolerance)

- [ ] **No `anyhow::anyhow!()`**: Use `AppError` or specific error types
- [ ] **No `unwrap()`**: Use `?` operator or `unwrap_or`
- [ ] **No `panic!()`**: Return `Result` instead
- [ ] **Structured errors**: Use `thiserror` with named fields

## Module Organization

- [ ] **Public API in mod.rs**: Re-export public items
- [ ] **Private implementation**: Keep internals private
- [ ] **Logical grouping**: Related functionality in same module
- [ ] **Feature flags**: Conditional compilation for database backends

## Documentation

- [ ] **Doc comments**: `///` for public items
- [ ] **Examples**: Include usage examples in doc comments
- [ ] **Error cases**: Document when functions return errors
- [ ] **Safety**: Document `unsafe` code (if unavoidable)

## Testing

- [ ] **Unit tests**: Test individual functions
- [ ] **Integration tests**: Test component interaction
- [ ] **Deterministic**: Use seeded RNG for reproducible tests
- [ ] **No external dependencies**: Use synthetic data, not OAuth

## Security

- [ ] **Input validation**: Validate all user inputs
- [ ] **SQL injection prevention**: Use parameterized queries
- [ ] **Secret management**: Never hardcode secrets
- [ ] **Secure memory**: `zeroize` for cryptographic keys

## Performance

- [ ] **Database pooling**: Reuse connections
- [ ] **Async operations**: Use `tokio` for I/O
- [ ] **Minimal cloning**: Only clone when necessary
- [ ] **Efficient algorithms**: Use appropriate data structures

## Key Takeaways

1. **Error handling**: Zero tolerance for `anyhow::anyhow!()` and `unwrap()`.
2. **Module organization**: Clear public API, private internals.
3. **Documentation**: Comprehensive doc comments with examples.
4. **Testing**: Deterministic tests with synthetic data.
5. **Security**: Input validation, parameterized queries, secret management.
