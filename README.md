# Cognos

**Deterministic control over non-deterministic computation.**

Cognos is a typed, imperative programming language for defining agentic workflows. It compiles to an internal representation executed by the Neocognos kernel, giving you precise control over AI-powered automation while keeping the unpredictable parts explicit and contained.

## Key Insight

The LLM is a **co-processor**. You call it via `think()`, but everything else—data flow, control flow, types—is deterministic, explicit, and verifiable. No more black-box agents. No more prompt engineering guesswork. Just clean code that happens to think.

## Hello World

```cognos
flow hello_world:
    input = receive(Text)
    response = think(input, system="Be helpful and concise.")
    emit(response)
```

This flow receives text input, asks the LLM to respond helpfully, and emits the result. Simple, explicit, and typed.

## Vision

We believe the future of AI systems is neither fully deterministic nor fully emergent. It's **hybrid**: deterministic scaffolding around carefully controlled non-deterministic computation.

Cognos makes that vision real:
- **Explicit**: Every LLM call is visible in your code
- **Typed**: Static analysis catches errors before runtime  
- **Composable**: Flows call other flows like functions
- **Verifiable**: Test your agentic logic like any other code
- **Production-ready**: Compiles to the same runtime that powers Neocognos

## What's Next

- Explore the [examples](./examples/) directory
- Read the [language specification](./spec/language-spec.md)
- Learn about [compilation](./spec/compilation.md)

The future of programming is agentic. Let's build it together.