# TLS transports

[Library index](README.md) - [Networking and peers](net.md)

`@"tls"` provides authenticated encrypted streams independently of HTTP. It is a
specified library contract, not implemented or security-qualified bootstrap support.
An implementation must qualify its provider, configuration and failure behavior
before an application can rely on this contract. Importing net or using net.http must not
silently choose a trust store, credentials or an unverified native provider.

## Versions, identity and trust

The baseline supports TLS 1.2 and TLS 1.3, preferring 1.3. SSL and TLS 1.0/1.1 are
not accepted. Cipher, group and signature profiles must follow the applicable
[secure TLS recommendations](https://www.rfc-editor.org/rfc/rfc9325.html), be
versioned with the provider profile, and remain inspectable. Applications choose a
minimum version but cannot request an obsolete profile through this API.

Client handshakes validate the chain and the intended service identity according
to [TLS service identity rules](https://www.rfc-editor.org/rfc/rfc9525.html).
The reference identity comes from the requested URL/service, not reverse DNS, an
HTTP response, a redirect's previous origin or an unchecked certificate field.
An IP-literal service needs a matching IP identity; it does not become a DNS name.
Initial host input is an ASCII DNS name/A-label or numeric address, with no implicit
host-locale or IDNA transformation.

Trust is explicit: either an application-supplied root set or an explicitly loaded
snapshot of the host trust store. Host trust loading can fail and is an observed
effect, not a pure default. Configurations retain the identity/version of their
trust snapshot. Additional pins narrow trust; they do not silently disable chain
or identity checks. Development certificates use explicit local roots, not a
`verify=false` option. This API provides no implicit plaintext fallback.

Zero-RTT application data is disabled. Session resumption does not enable replayable
early requests. ALPN offers and accepted results are explicit; an unoffered or
unsupported selection fails rather than guessing an application protocol.
Server client-certificate authentication is an explicit policy, separate from
application authorization. Validating a certificate does not log a user in.

## Configuration and operation surface

| API/type                                                 | Contract                                                                         |
| -------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `tls.roots(bytes, allocator, limits)`                    | Parse an owned root set or return a typed configuration/allocation failure       |
| `tls.system_roots(allocator, limits)`                    | Explicitly snapshot supported host trust data or fail                            |
| `tls.identity(chain, key, allocator, limits)`            | Construct an owned credential after format/key consistency checks                |
| `tls.client_config(spec)`                                | Validate versions, trust, optional client identity, ALPN and handshake limits    |
| `tls.server_config(spec)`                                | Validate server identities, client-auth policy, ALPN and handshake limits        |
| `tls.connect(stream, config, peer, allocator, deadline)` | Consume a connected transport and authenticate the named service                 |
| `tls.accept(stream, config, allocator, deadline)`        | Consume an accepted transport and perform the configured server handshake        |
| `stream.read(buffer)`, `stream.write(bytes)`             | Authenticated stream I/O with the ordinary partial-progress contract             |
| `stream.peer()`, `stream.protocol()`                     | Borrow verified peer facts and the negotiated protocol/version                   |
| `stream.close(deadline)`                                 | Consume the owner, attempt bounded orderly shutdown, and report failure/progress |

Configurations are immutable owners or views of explicitly retained configuration
owners. A configuration retaining roots, credentials or allocator storage cannot
outlive them. Handshakes and streams keep those lifetimes visible. Creating a
configuration performs no DNS lookup, connection or handshake.

A successful handshake returns a move-only Stream. Failure closes the consumed
transport and releases partial state; no partly authenticated stream escapes.
Retrying another address requires a new transport. Providers may use host resources,
but all growing language-owned state uses the supplied allocator and declared limits.

Streams require exclusive access for mutable I/O unless a separately documented
connection driver provides synchronized ownership. They implement the existing
`io.Read`/`io.Write` progress rules. A successful write counts accepted plaintext,
not proof that an application at the peer processed it. Authentication failures
never expose unauthenticated plaintext as a successful prefix.

Unexpected transport EOF without authenticated TLS closure is distinguishable
from orderly end-of-stream. An upper protocol with independently complete framing
may decide whether its message was already complete; it cannot erase the TLS
failure or reuse the closed connection. Dropping an owner releases resources but
does not claim that a graceful close notification reached the peer.

## Bounds, cancellation and failures

Configuration requires finite bounds for handshake bytes/work, certificate bytes
and chain length, buffered records, concurrent handshakes and cached sessions.
Invalid limits are configuration failures, not unlimited defaults. Deadlines are
absolute monotonic instants. Cancellation follows the task unwind contract and
must release partial handshake/record state before the task can finish joining.

Errors retain a portable kind and phase, such as Configuration, TrustUnavailable,
IdentityMismatch, CertificateRejected, ProtocolVersion, Negotiation, Integrity,
Truncated, TimedOut, Transport or Allocation. Peer alerts/provider codes are
optional diagnostic facts, not stable application error identities. No error
contains private key material, traffic secrets or an unbounded peer certificate dump.
Application code uses concrete errors and the standard explicit erasure rules.

Private keys, ticket keys and traffic secrets are not ordinary diagnostic/replay
payloads. Providers must minimize copies and clear their controlled secret buffers;
this is not a promise to scrub arbitrary caller copies, registers or crash dumps.
Key-log export, if ever added, requires a separate explicit security contract.
No ambient environment variable enables it through this interface.

## Qualification boundary

The initial provider must be an established, reviewed implementation behind a
private adapter, not newly invented cryptography. Pin its version, build options,
license and update policy. Verify certificate/identity failures, explicit roots,
ALPN, resumption without early data, truncation, partial I/O, cancellation, resource
limits and secret redaction using controlled peers. Record tested host/provider
combinations and unavailable capabilities honestly.

DTLS, QUIC, custom insecure profiles, dynamic credential callbacks and TLS record
replay are not implied by this stream contract. [net.http](http.md) consumes a qualified
TLS transport through the peer's configured transport pipeline. Sender and receiver
roles require their respective client/server trust and identity policies explicitly;
having both roles never reuses server credentials as client trust. Raw TCP/UDP and
plaintext HTTP do not imply TLS. Defining these APIs does not prove HTTPS works.
