# Networking and capability-typed peers

[Library index](README.md) - [I/O](io-and-system.md) - [HTTP protocol](http.md) - [TLS](tls.md)

`@"net"` is the networking package: addresses, DNS, TCP, UDP, transport management
and protocol-aware peers. `net.http` is an ordinary namespace in that package,
not a second import or a compulsory router framework. Reachability selects the
helpers and runtime services actually used; raw networking does not link HTTP
parsers, codecs, routers or TLS providers merely because they share a package.

This is a specified library contract, not implemented bootstrap functionality.
It replaces the earlier separate HTTP package design without a compatibility
package. No networking execution, security qualification, new diagnostic code or
public artifact format is established by this documentation change.

## Layers and responsibilities

| Layer                    | Responsibility                                                              |
| ------------------------ | --------------------------------------------------------------------------- |
| Addresses and resolution | Numeric addresses, endpoints, explicit DNS and deadlines                    |
| Transports               | TCP byte streams, UDP datagrams and their resource/progress rules           |
| Peer composition         | Typed transport, protocol and sender/receiver roles; admission and lifetime |
| Protocol adapters        | Message framing, codecs and protocol-specific operations                    |
| HTTP contracts           | Typed requests/replies, optional routers, middleware and response policies  |

`net.peer()` composes these layers; it does not replace the direct transport APIs.
TCP preserves byte order, not message boundaries. UDP preserves datagram boundaries,
not reliable delivery, ordering or request/response correlation. Neither gains
framing, acknowledgements, retries or schemas from the peer wrapper. Those rules
belong to an explicitly selected protocol or application contract.

## Capability-typed configuration

A peer's concrete type is derived from its transport, protocol and configured
roles. Roles are capabilities, not runtime Boolean flags, string names or nullable
callbacks. Configuration methods consume their builder and return its next concrete
type, retaining concrete handler/capture types without implicit boxing or erasure.

| Configuration operation              | Contract                                                                           |
| ------------------------------------ | ---------------------------------------------------------------------------------- |
| `net.peer()`                         | Create an unstarted builder with no transport or roles                             |
| `net.tcp(config)`                    | Describe a TCP transport and explicit outgoing/bind policy without opening sockets |
| `net.udp(config)`                    | Describe a UDP transport and explicit local binding/destination policy             |
| `spec.transport(transport)`          | Select the single concrete transport pipeline                                      |
| `net.raw.sender(config)`             | Describe native stream initiation or datagram sending for the selected transport   |
| `net.raw.receiver(handler, config)`  | Bind a concrete accepted-stream or incoming-datagram handler                       |
| `net.http.sender(config)`            | Describe HTTP request initiation, response decoding and bounded pooling            |
| `net.http.receiver(service, config)` | Bind a concrete HTTP service and its exchange/admission policy                     |
| `spec.sender(sender)`                | Add the one outbound role and its concrete protocol adapter                        |
| `spec.receiver(receiver)`            | Add the one inbound role and its concrete protocol adapter                         |
| `spec.start(allocator, limits)`      | Consume a complete configuration and return an active peer or StartFailure         |

There must be exactly one transport and at least one role before start is available.
Transport, sender and receiver may be configured in either order; completing the
builder checks their compatibility. Duplicate transport/role registration is a
construction error, not replacement of an earlier handler. Multiple HTTP endpoints
are composed into one service/router rather than registered as multiple receivers.

Structural incompatibilities and calls to unavailable operations fail checking.
For example, a receiver-only peer cannot initiate an independent HTTP request,
and a raw UDP peer has no HTTP call operation. HTTP's initial adapters require a
qualified stream transport; UDP is not silently upgraded into an HTTP transport.
A combined peer requires compatible sender/receiver protocol and transport types.
Separate peer values can use different protocols; no universal dynamic peer is
introduced to hide incompatible configurations.

The sender role initiates independent outbound activity. The receiver role accepts
incoming activity and invokes its handler. They do not mean write-only and read-only
sockets: an HTTP sender receives responses, and an HTTP receiver sends replies.
A receiver can reply within its admitted exchange without gaining permission to
initiate unrelated exchanges. Configuring both roles supports gateways and services
that also make outgoing requests; it does not automatically forward anything or
make HTTP connections bidirectional request channels.

These are ordinary library values and type-directed calls, not new grammar or
compiler recognition of familiar names. Runtime addresses, limits and credentials
still require value validation. Invalid configuration produces a typed failure;
it does not turn into an unlimited default. Stable diagnostics for unavailable
capabilities must be assigned when the feature is implemented.

## Protocol-specific operations

| Active peer configuration      | Available activity                                                                        |
| ------------------------------ | ----------------------------------------------------------------------------------------- |
| TCP with raw sender            | `peer.connect(endpoint, deadline)` returns a bounded StreamLease with ordinary stream I/O |
| TCP with raw receiver          | Dispatch an accepted StreamLease to the configured handler                                |
| UDP with raw sender            | `peer.send_to(endpoint, bytes, deadline)` submits one complete datagram or fails          |
| UDP with raw receiver          | Dispatch a DatagramEvent with sender, bytes, truncation and a scoped reply operation      |
| HTTP with sender               | `peer.send(request, options)` and `peer.call(endpoint, input, allocator, options)`        |
| HTTP with receiver             | Dispatch decoded exchanges to the configured concrete service                             |
| Compatible sender and receiver | Union of their operations, with shared peer ownership and explicit budgets                |

A raw stream lease has the same partial read/write and half-close rules as
`net.Stream`, but retains its peer's lifetime and admission budget. A datagram event
retains its receive buffer; borrowed data and its reply capability cannot outlive
the admitted handler work. Copying data into an explicit owner does not extend the
reply capability. A successful datagram send is not evidence that a remote handler
received or processed it. Raw peers have no invented Request/Response envelope.

HTTP's concrete request, response, body and effective endpoint types belong to
`net.http`. One sender can call different typed endpoints: each call derives its
input and response alternatives from that endpoint, rather than fixing one response
type for the entire peer. Wire/transport failures remain distinct from declared
application responses. An HTTP receiver may bind a plain handler or a checked
router; routing is optional composition, not a requirement for a peer.

## Startup, ownership and shutdown

Builder methods configure values; they do not resolve DNS, bind, connect, accept,
start background tasks or execute handlers. Runtime validation occurs before
startup effects. Explicit earlier helpers such as loading trust roots or allocating
configuration data retain their own documented effects and owners.

`start` uses the supplied allocator and finite resource limits, binds configured
receivers and establishes bounded driver/admission state. Outbound connections and
DNS occur when an operation requires them; startup does not make a hidden request.
StartFailure returns the configuration and a concrete error after closing partial
resources and joining partial driver work. No handler is dispatched from a partly
started peer. Cancellation follows ordinary task unwinding rather than returning
a fabricated startup result.

An active peer is move-only. Protocol adapters define the exact transfer/sharing
capabilities of their handles; having both roles does not grant Send/Sync to
non-shareable captures. HTTP send operations use synchronized pool state and may
share a peer only under those capabilities. Leases, handler tasks and captured
owners remain live until their last use; no socket or task becomes detached by
dropping a request handle. Limits account separately for sender pools, receiver
admission, queues, protocol state and task stacks, with a finite total peer budget.

`peer.stop(deadline)` closes admission to new independent sends and incoming work,
then drains existing operations and handler replies under an absolute monotonic
deadline. Later stop requests may shorten but never extend that deadline. Expiry
requests cancellation and joining; it does not release borrowed storage or detach
non-cooperative work. Shutdown reports when work delays joining. Dropping an owner
uses the ordinary structured cleanup contract, not an implicit successful drain.

`peer.close()` consumes a stopped, quiescent peer after outstanding leases and child
tasks have ended. Stop/close apply to both roles together; closing a receiver does
not secretly leave its sender running. Separate lifetimes require separate peers.
A stopped peer cannot restart or gain another role in place; construct a new
configuration instead. A retained handle cannot acquire operations absent from its
static capabilities, even when a runtime configuration would happen to allow them.

The following is a composition fragment with caller-supplied, validated transport,
sender and receiver configuration plus a concrete service; it is not a runnable
bootstrap program:

```meowy
net : @"net"

spec : net.peer()
    .transport(net.tcp(transport))
    .sender(net.http.sender(outgoing))
    .receiver(net.http.receiver(service, incoming))

started : spec.start(allocator, limits)
```

The caller must handle the startup result before using the active peer. Omitting
the sender or receiver builder call produces a different capability type, not a
disabled role checked at each operation.

## Direct network services

`net.parse_address(text)` returns an inline IPv4/IPv6 `net.Address` or
`net.InvalidAddress`. It parses numeric addresses only, without DNS or a default
port. `net.endpoint(address, port <uint16>)` combines values without opening a
socket. `net.scoped_endpoint(address, port, scope <uint32>)` additionally selects
an IPv6 interface index and returns Endpoint or InvalidAddress; zero means no
scope and a nonzero scope on IPv4 is invalid. Interface indices belong to the host.

| API                                                    | Result                                                       | Contract                                                           |
| ------------------------------------------------------ | ------------------------------------------------------------ | ------------------------------------------------------------------ |
| `net.resolve(name, port, allocator, deadline)`         | `net.Addresses` or `net.Error` or `memory.AllocationFailure` | Resolve a host with an explicit nullable monotonic deadline        |
| `net.connect(endpoint, deadline)`                      | `net.Stream` or `net.Error`                                  | Connect a TCP stream, without implicit retries to another endpoint |
| `net.listen(endpoint, backlog <uint32>)`               | `net.Listener` or `net.Error`                                | Bind and listen; zero port lets the host select one                |
| `listener.local_endpoint()`                            | `net.Endpoint`                                               | Read the actual bound address and port                             |
| `listener.accept(deadline)`                            | `net.Stream` or `net.Error`                                  | Wait for one accepted stream                                       |
| `stream.read(buffer)`, `.write(bytes)`                 | `io.Read`, `io.Write`                                        | Byte-stream I/O; message boundaries are not preserved              |
| `stream.deadline(instant <time.Instant><null>)`        | `null`                                                       | Set/clear the stream's read and write deadline                     |
| `stream.shutdown_write()`                              | `null` or `net.Error`                                        | Send an orderly write-side shutdown while retaining reads          |
| `net.datagram(endpoint)`                               | `net.Datagram` or `net.Error`                                | Bind a UDP endpoint                                                |
| `socket.receive(buffer, deadline)`                     | Datagram result or `net.Error`                               | Return sender, count, and explicit truncation status               |
| `socket.send(destination, bytes, deadline)`            | `null` or `net.Error`                                        | Submit one complete datagram or fail                               |
| `stream.close()`, `listener.close()`, `socket.close()` | `null` or `net.Error`                                        | Consume the owner and release its host resource                    |

Network owners are move-only. Blocking waits are task cancellation points;
network runtime support suspends the waiting task rather than occupying a worker
for the whole wait. Without an executor they block the calling host thread.
Explicit network deadlines return TimedOut errors; cancellation inherited from a
task follows the task's unwind contract. Successful stream data already read or
written remains progress even if the deadline then expires.

Addresses owns its resolver results and exposes a borrowed slice; DNS order is
not promised stable across calls. A truncated datagram reports the bytes retained
and `truncated : true`; the discarded suffix cannot be recovered by another read.
A zero-length datagram is a message, not end-of-stream. None of these APIs infers
TLS, an application protocol, or an encoding from a port number.

[TLS](tls.md) supplies a separate authenticated stream contract with explicit trust
and identity policy. [net.http](http.md) layers messages, sender/receiver adapters, streaming exchanges
and typed route/response contracts over qualified transports. Those specified APIs
do not imply that the bootstrap currently implements network protocols or TLS.

## Transport security and qualification

TLS remains the explicit stream-security contract in `@"tls"`. A qualified peer
transport pipeline can wrap its TCP connections with the existing client/server
TLS configuration and handshake rules. Outbound identity must be supplied by the
protocol or explicit transport policy; it is not inferred from a port number.
Combined roles retain separate client and server trust/identity policies. Plain
TCP, UDP and HTTP neither select a TLS provider nor silently enable encryption.

Qualification starts with direct transports, then typed peer composition, raw
adapters and lifecycle, then protocol adapters. Test unavailable operations and
incompatible roles at checking time; test invalid runtime configuration, allocation
failure and partial startup at execution time. Exercise sender-only, receiver-only
and combined peers, concurrent calls, reply lifetimes, truncation, exhausted
admission, cancellation and join-before-release in supported profiles.

HTTP framing, authentication, versions, typed schemas and generated client/docs
agreement retain the separate [HTTP qualification](http.md#implementation-and-qualification)
and [TLS qualification](tls.md#qualification-boundary) requirements. Passing an
in-memory peer test does not qualify sockets, encrypted transports or shutdown on
the supported host. Unavailable adapters must stay explicitly unavailable.

Network observations follow [recorded effects](io-and-system.md#recorded-effects).
This merger adds no replay event IDs, packet recording, secret-capture policy or
promise that a live network peer is deterministic.
