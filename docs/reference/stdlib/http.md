# HTTP protocol for net peers

[Library index](README.md) - [Networking and peers](net.md) - [I/O](io-and-system.md) - [TLS](tls.md)

`net.http` is the HTTP protocol namespace of `@"net"`, not a separate package or
a compulsory web framework. Messages, sender/receiver adapters, streaming codecs
and contract-checked routers are separately usable layers. All named constructors
are ordinary library values. Importing one layer retains only reachable helpers
and runtime services.

[Capability-typed peers](net.md#capability-typed-configuration) own transport,
roles and lifecycle. An HTTP sender initiates requests and receives responses;
an HTTP receiver handles requests and sends replies. Either role or both can be
configured on `net.peer()`. HTTP adds no alternate client factory or serving loop.

This is a specified library contract, not implemented bootstrap functionality.
Its baseline targets HTTP/1.1 and HTTP/2 over qualified transports; implementations
must advertise only versions they actually implement and test. HTTP/3, QUIC and
WebSocket framing are separate work, not aliases for supported HTTP/2 streams.
No new compiler/runtime support, native provider or public artifact schema is
established merely by adding this chapter.

## Layers and protocol boundaries

| Layer            | Responsibility                                                                             |
| ---------------- | ------------------------------------------------------------------------------------------ |
| Messages         | Methods, URLs, status values, ordered fields, body streams and trailers                    |
| Sender adapter   | Origin selection, authenticated connections, bounded pooling and explicit attempt policies |
| Receiver adapter | Connection/stream admission, exchanges, deadlines, cancellation and draining               |
| Codecs           | Bytes, text, typed JSON, forms, multipart and explicit streaming representations           |
| Contracts        | Typed inputs, named response alternatives, middleware context and effective wire schemas   |
| Router           | Deterministic matching and dispatch through those contracts                                |

The protocol models follow [HTTP semantics](https://www.rfc-editor.org/rfc/rfc9110.html).
HTTP/1.1 framing and connection handling follow
[RFC 9112](https://www.rfc-editor.org/rfc/rfc9112.html); the multiplexed profile follows
[RFC 9113](https://www.rfc-editor.org/rfc/rfc9113.html). Library policies below may
reject ambiguous input more strictly; they must not reinterpret invalid framing
as another valid request.

Method names are case-sensitive ASCII tokens. Standard method values such as
`net.http.Get` and `net.http.Post` are ordinary constants; extension methods remain possible
through checked construction. Status values are validated protocol values, not
arbitrary integer casts. Informational messages and upgrades are separate from a
handler's final response.

URLs retain scheme, authority, path and query separately. Client URLs require an
explicit `http` or `https` scheme and valid authority; fragments never enter the
request target. Userinfo is rejected rather than becoming implicit credentials.
Initial parsing accepts URI syntax with ASCII DNS/A-label hosts. Segment builders
percent-encode UTF-8 explicitly; they do not normalize Unicode or consult a locale.

Header names compare case-insensitively, but headers are an ordered multimap, not
a string dictionary. Values are validated bytes and are not assumed to be UTF-8.
CR, LF, NUL and invalid control bytes cannot be injected through constructors.
Field-specific parsing controls combination; in particular, Set-Cookie values are
not joined into one comma-separated value. Incoming raw spelling/order can remain
available without controlling canonical matching.

## Storage and one-shot bodies

`Request<B>` and `Response<B>` keep a concrete body type B. A body can be an empty
value, a borrowed byte view, an owned buffer/file, or a concrete reader/callable
with its own captures. There is no mandatory boxed body, universal dynamic context
or implicit whole-message buffer. Borrowed headers and body views retain their
owners; queues and concurrent tasks cannot erase those lifetimes.

An incoming body is a one-shot, move-only stream. Reading requires exclusive access
to its cursor; decoding or collecting consumes that cursor. A second decoder does
not rewind it. Explicit buffering can produce a reusable owned value, under a
declared limit and allocator. Replaying an outbound body requires a fresh-body
factory or an explicitly rewindable owner, not a Copy claim on a consumed stream.

Reads and writes use `io.Read`/`io.Write` progress semantics. EOF, trailers, partial
progress and failure remain distinguishable. Trailers become available only after
the body finishes and cannot change framing, authentication or routing decisions
that were already made from the head. Backpressure propagates to producers; a slow
peer does not trigger an unbounded background buffer.

A client response holds a connection/stream lease. It must be consumed, explicitly
closed or dropped. Unread HTTP/1.1 data prevents pool reuse unless an explicitly
bounded drain succeeds. HTTP/2 may reset the affected stream while retaining a
healthy connection. Destruction releases the lease without silently draining an
unbounded body. Response views cannot outlive their response/lease owner.

A peer with an HTTP sender is an opaque move-only owner with synchronized request
operations, usable concurrently under its concrete Send/Sync contract. Outstanding
leases retain its lifetime; it cannot be destroyed while they borrow it. Pool
state, queues and protocol drivers are charged to the peer's explicit allocator
and resource budgets. A receiver's captures may further restrict sharing when both
roles are configured. No task or socket becomes detached merely because a request
handle was dropped.

## Sender surface and attempt policy

| API                                              | Contract                                                                                    |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------- |
| `net.http.url(text)`                             | Validate a borrowed URL view or return InvalidUrl                                           |
| `net.http.headers(allocator, limits)`            | Create an explicitly owned bounded header collection                                        |
| `net.http.request(method, url, headers, body)`   | Construct a typed message without sending it                                                |
| `net.http.sender(config)`                        | Describe the HTTP sender role without opening a connection or making a request              |
| `spec.sender(sender)`                            | Attach that role to a capability-typed net peer configuration                               |
| `peer.send(request, options)`                    | Consume an outbound message and return a leased response or a payload-retaining SendFailure |
| `peer.call(endpoint, input, allocator, options)` | Encode/decode through an effective endpoint contract; unavailable without an HTTP sender    |
| `peer.stop(deadline)`, `peer.close()`            | Use the shared peer shutdown contract after response leases and request tasks end           |

HTTP sender configuration requires finite pool/admission limits and explicit proxy,
trust and protocol policies. HTTPS uses the separate TLS contract. DNS and address
selection have bounded attempts and deadlines. Environment proxy variables, cookie
stores, credentials, response caches and cross-origin connection coalescing are
not implicitly enabled. Origin identity includes scheme, host and effective port;
pool partitions also respect proxy, trust and client-identity configuration.

One attempt and no automatic redirects are the default. An explicit redirect policy
sets a finite hop limit, method rewriting and allowed destinations. Cross-origin
redirects strip authorization and origin-scoped credentials unless separately
authorized. HTTPS-to-HTTP downgrade is rejected by default. Replay-preserving
redirects cannot resend a consumed body without a fresh-body factory.

Retries need a finite attempt/backoff policy, suitable method/application semantics
and a replayable body. A timeout does not prove that the peer did nothing. Generic
idempotency-key headers do not authorize retries without an application contract.
SendFailure retains the request with its current body cursor, a concrete error,
committed/read progress and whether delivery is known not to have started or may
have occurred. A partially consumed cursor is not the original replayable request.
Failures before acquisition preserve an unconsumed payload; cancellation instead
uses the ordinary task unwind/cleanup contract.

An unexpected peer status or media type does not get cast into the nearest typed
response. Typed calls return an UnexpectedResponse retaining the raw response for
bounded inspection or closure. Decoder failures remain distinct from transport
failure and from a valid application error response.

## Receiver exchanges and bounded shutdown

`net.http.receiver(service, config)` describes a concrete service and its HTTP
exchange policy. Attaching it with `spec.receiver(receiver)` enables incoming
exchanges when that peer starts. The selected transport owns binding and connection
admission; the adapter does not create a separate listener owner or serving loop.
A receiver-only peer can send handler replies without a sender role, but cannot
initiate independent requests. The same service can run in an in-memory test adapter
without starting a network peer.

`peer.stop(deadline)` supplies the explicit absolute monotonic drain deadline.
The first stop begins draining; later requests may shorten, not extend, its deadline.
Both roles use the shared [peer lifecycle](net.md#startup-ownership-and-shutdown);
there is no separate HTTP Stop channel or detached server lifetime.

Service and captured state retain concrete types. Concurrent access requires the
normal transfer/sharing capabilities; exclusive state cannot be smuggled into a
shared string-keyed context. Receiver configuration accounts for connection, stream,
queue, task-stack and handler-admission bounds, including its control/driver tasks.
Construction/dispatch never creates an unbounded task per buffered request.

The peer's receiver owns each Exchange through request decoding, handler work, response
writing and cleanup. A handler can borrow its request data for serialization while
that exchange remains alive. Returning a borrow does not detach it from the request
or allocator. Handler-created owned streams transfer into the response writer;
their captures must survive transmission, not just the handler's return.

Stopping first closes admission, then drains admitted exchanges under the earliest
deadline. Expiry requests cancellation of remaining work and joins it before owners
are released. A deadline is not permission to destroy borrowed storage or leave
unjoined tasks running. Non-cooperative work can delay shutdown and must be reported.
Task cancellation, domain failures and recoverable panics follow existing runtime
contracts; HTTP does not create an independent exception/unwind model.

Before the final head is committed, policy can select a typed error response.
After commitment, write/stream failures terminate or reset the exchange and report
partial progress. They cannot append a second JSON error or change the status.
Fatal termination and a lost connection do not promise any response at all.

## Limits and defensive parsing

Sender/receiver configuration requires a complete Limits record and deadline policy;
there is no omitted-field meaning of unlimited. Deployments may publish named
profiles, but those profiles must specify their exact versioned values.

| Budget                                                         | Required boundary                                                        |
| -------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Target/start line and wire head                                | Count bytes before unbounded parsing or allocation                       |
| Field count and decoded field bytes                            | Bound names, values and compression expansion independently of wire size |
| Encoded body, decoded body and buffered body                   | Separate finite counters; a stream is not permission for unlimited work  |
| Content-coding layers and decoder work                         | Reject excessive nesting/expansion before oversized output allocation    |
| Trailer bytes and fields                                       | Separate bounded post-body metadata                                      |
| Multipart parts, headers and field/file bytes                  | Apply both total-message and per-part limits                             |
| Connections, streams, queues and pool entries                  | Bound live resource admission, not just completed payload sizes          |
| Connect, TLS, head, idle-progress and total exchange deadlines | Use monotonic time; an idle timeout does not replace a total deadline    |

These acceptance limits are not a byte-accurate allocator quota. Allocator overhead,
protocol tables and task stacks need their own resource accounting. Invalid limits
are ConfigurationError values; external input exceeding a limit is a bounded
request/response failure. No configuration is selected from host memory size.

The HTTP/1.1 profile rejects bare-LF parsing, obsolete folding, whitespace before
a field colon, conflicting/duplicate Content-Length fields, and simultaneous
Transfer-Encoding and Content-Length. It rejects unsupported transfer codings and
incomplete framing rather than trying a second interpretation. After a framing
error the connection is not returned to the pool. A proxy must parse and serialize
validated messages, not forward ambiguous raw framing unchanged.

HTTP/2 qualification additionally covers bounded compressed/decoded field blocks,
continuations, stream state, flow control, cancellation, connection versus stream
errors and graceful connection shutdown. Negotiation alone is not HTTP/2 support.
Default clients do not pipeline HTTP/1.1 requests, and push is not an implicit
server/client feature of this API.

## Body codecs and ownership

| Descriptor                                       | Decoded/encoded representation                                              |
| ------------------------------------------------ | --------------------------------------------------------------------------- |
| `net.http.empty()`                               | No representation body                                                      |
| `net.http.bytes()`                               | Explicit byte owner/view or bounded body stream, selected by the operation  |
| `net.http.text()`                                | Validated UTF-8 with an explicit owner when collected                       |
| `net.http.json<T>()`                             | Existing typed JSON schema and a `json.Document<T>` on decode               |
| `net.http.form<T>()`                             | Bounded URL-encoded fields through declared scalar parsers                  |
| `net.http.multipart(spec)`                       | Bounded streaming parts with explicit per-part descriptors                  |
| `net.http.map_codec<T, U>(base, encode, decode)` | Checked mappings between domain T and the base codec's declared wire type U |

Only supported [JSON schemas](json.md#typed-schemas-and-numbers) are accepted by
`net.http.json<T>()`. HTTP does not add arbitrary JSON union decoding, annotations or
wire-name rewriting. A response's status/media alternative selects its own codec,
so different statuses can decode different concrete JSON records without a second
JSON type system. A mapped codec encodes T into the base codec's checked U value,
not arbitrary bytes. Decode first validates U, then applies the domain mapper;
borrowed mapped values retain the base decoded owner. Arbitrary validator semantics
cannot be inferred into an exact schema. Raw custom byte encoders belong to the
low-level service boundary, not a strict router merely because they assert a schema.

A decoded JSON request retains its Document for the entire required exchange
lifetime. `body.root()` borrows that owner. A fully collected typed client result
can own its Document and release the transport lease, but cannot outlive its
allocator; this narrower storage contract must be implemented explicitly, not
inferred for arbitrary wrappers. No `take_root` escape from borrowed JSON fields
is added by HTTP.

Media type/charset and content-coding checks precede typed delivery. Automatic
decompression is an explicit policy with encoded and decoded budgets. Multipart
does not create temporary files automatically or trust peer filenames as paths.
File upload sinks are caller-supplied bounded writers. Cookies, authorization
schemes, ranges, conditional requests, event streams and proxy forwarding are
composable helpers over messages; no cookie jar, login, cache or forwarding trust
is inferred by constructing a router.

## Describe a typed route

The following is source using the specified API, not a bootstrap-executable example:

```meowy
net : @"net"

#| A public user representation. |#
<User> : <{ id <uint64>; name <string> }>
<Missing> : <{ code <string>; message <string> }>
<UserPath> : <{ id <uint64> }>

find : net.http.route({
    -> method : net.http.Get
    -> path : "/users/{id}"
    -> params : net.http.fields<UserPath>()
    -> responses : {
        -> found : net.http.reply(200, net.http.json<User>())
        -> missing : net.http.reply(404, net.http.json<Missing>())
    }
})

#| Finds a user from a decoded, validated identifier. |#
handle <find.Reply> : (request <&find.Request>) {
    -> find.reply.found({ -> id : request.params.id; -> name : "Mochi" })
}

app : net.http.router({ -> routes : { -> get_user : find } })
service : app.bind({ -> get_user : handle })
```

`net.http.route`, codec descriptors and `net.http.router` are pure compile-time constructors
over closed data, like the CLI library's descriptors. Construction checks names,
patterns, input/response types and composition without opening sockets or running
handlers. `app.bind` checks concrete callable signatures/captures; it does not
perform a request. No annotations, runtime reflection or automatic callback boxing
are introduced. Construction failures must be explicit diagnostics, with assigned
codes added to the central catalog when this feature is implemented.

Request has typed `params`, `query`, selected `headers` and `body` accessors. Omitted
parameter/query/header descriptors describe empty records; omitted body means
`net.http.empty()`. Path parameters are required and non-nullable. Query fields are
required unless nullable or given a typed default. An optional context descriptor
declares an additional handler argument; without it the handler takes only Request.
The fixture handler above demonstrates typing, not a database or a universal 200.

Named reply constructors produce the route's closed Reply alternatives. Their
status, media type, body codec and declared headers are fixed by the descriptor.
The caller cannot substitute an arbitrary Response or forge that identity with a
record. A typed handler receives no raw response-writer escape hatch.
Undeclared statuses, wrong payloads and missing middleware context fail
checking. Runtime validation is still required for all incoming network data.

`net.http.fields<T>()` supports closed records of string, Boolean and integer fields,
nullable query fields and explicitly bounded repeated query fields. Wire names
match declared field names unless an explicit descriptor mapping is supplied.
Default integers are ASCII decimal with an optional minus only for signed types;
plus signs, whitespace, separators, prefixes, fractions and exponents are rejected.
Booleans are exactly `true` or `false`; strings require valid decoded UTF-8.
Header descriptors similarly map typed members to validated header names; they
cannot override framing/authority fields owned by the protocol driver.
Custom scalar parsers are concrete pure callables with typed failure results and
visible borrowed-input lifetimes. No string-keyed untyped result bag is created.

Unknown query fields and duplicate scalar parameters are errors by default; repeated
fields require an explicit bounded descriptor. Query/form decoding maps `+` to
space and percent-decodes once, then validates UTF-8. Raw URL/query access preserves
the original bytes. Typed path matching splits before decoding, rejects malformed
escapes and encoded separators, and does not collapse slashes, case or dot segments.
Trailing slash is significant. Parse failure is not a reason to try another route.

The initial typed route grammar has literal segments and `{name}` segments. A
literal specialization can take precedence over a broader parameter pattern;
other intersecting patterns are rejected at construction. Registration order,
custom parser success, regex backtracking and runtime mutation do not select a
winner. Regex/catch-all dispatch can use an explicitly raw service until separately
specified, not silently weaken a schema-enforcing router.

## Response policies and typed middleware

A route's effective contract includes handler replies and all applicable parsing,
validation, negotiation, authentication, authorization and middleware outcomes.
Middleware transforms concrete context types and either continues or produces a
declared outcome. A handler requiring authenticated context cannot be bound to a
pipeline that supplies only an optional or unverified identity.

Router construction checks the entire pipeline. Every fault mapper is total over
its declared fault alternatives. Conflicting status/media schemas require one
agreed wire alternative or fail construction; they do not become a dynamic union
the client must guess. Faults for unmatched paths or methods have a separate
router-level contract, since no endpoint matched them.

The default policy has empty-body protocol error replies, not an undocumented HTML
page. A policy can instead impose a shared typed envelope across the router.
It supplies pure type/codec construction plus checked runtime success/failure
mappers. Every applicable route and middleware response is composed through it,
and the effective client/OpenAPI contract describes the wrapped wire type.
Custom domain failures must be mapped; they are not automatically serialized.

For example, an application's success envelope can contain typed `data` and a
request identifier, while its error envelope contains a stable code and safe
message. A stricter application can choose one nullable-data record shape for
both. The policy must prove construction/codec compatibility; naming a type
ApiReply does not enforce mutual-exclusion invariants by itself.

The policy defines mappings for malformed typed inputs (400), unauthenticated or
forbidden access (401/403), unmatched routes/methods (404/405), negotiation failures
(406/415), size limits (413), admission/rate limits (429/503), and sanitized internal
failures (500). Rate-limit scope/storage and authentication remain application
policies, not hidden IP counters or string-keyed globals. Method rejection includes
the appropriate Allow metadata. Diagnostic stacks and secrets are never a default
error body. Panics are handled only at the existing recoverable task boundary.

HTTP body rules take precedence over envelopes. HEAD transmits no body; an automatic
HEAD mapping may reuse the GET handler's metadata but must not poll its body producer.
Explicit HEAD routes override that mapping. Final 204, 205 and 304 alternatives
require an empty body codec; representation metadata is not confused with bytes
transmitted. Upgrades/tunnels and raw services are explicit policy boundaries.
A schema promise cannot manufacture JSON after an invalid pre-request handshake,
exhausted allocation, committed response, cancelled exchange or broken connection.

## One contract for clients, tests and documentation

Effective endpoints expose concrete client input and Received alternatives.
Received owns each codec's decoded storage; JSON alternatives own Document, not
an unowned T extracted from it. Typed clients validate status, media type, declared
headers and body under the same limits/codecs. Unexpected wire values remain errors
with inspectable raw data, never successful schema casts.

`net.http.openapi(app, writer)` streams an [OpenAPI 3.2.0](https://spec.openapis.org/oas/v3.2.0.html)
description of effective exported routes, inputs, media/status alternatives,
middleware outcomes and response policies. Unsupported codec schemas fail export;
they do not produce invented descriptions. Router-level faults and raw service
boundaries are documented explicitly rather than inserted into every unrelated
operation. OpenAPI does not prove arbitrary application validator semantics.

The documentation builder can pair this descriptor with
[checked declaration comments](../documentation.md) for operation/type descriptions
and examples. It uses the same compiler snapshot and symbol identities, not a
second annotation parser. Ordinary runtime code gains no source-documentation
reflection. Binding an undocumented handler does not change its response types.
Export selection and any serialized intermediate metadata require their own tool
contract before a release claims compatible tooling.

Test adapters run an effective endpoint in memory with controlled reader/writer,
clock and transport inputs. They validate fault policy and response schemas as
well as successful handlers. An in-memory exchange is not evidence for socket,
TLS, HTTP/2, backpressure or shutdown behavior on the supported host.

## Implementation and qualification

1. Implement the shared net peer capability/startup contract before either HTTP
   role. Pin message/URL/header rules, error ownership and finite configuration limits.
   Build incremental parser/serializer fixtures and malformed framing regressions.
2. Qualify TLS separately, then sender leases, partial failures, bounded pooling,
   redirects/retries, cancellation and deterministic fixture transports.
3. Qualify receiver admission, response commitment, slow peers, body/trailer limits,
   handler state capabilities, streaming cleanup and join-before-release shutdown.
4. Add compile-time route/context/response-policy checks and reuse JSON owners.
   Test malformed inputs and middleware/global faults, not only 200 responses.
5. Qualify HTTP/2 negotiation, stream lifecycle, flow control and failure isolation.
   Keep HTTP/3, unsupported upgrades and unavailable providers explicitly gated.
6. Test generated client/OpenAPI agreement, safe errors, exact response bytes and
   doc bindings. Add assigned diagnostics and implementation evidence before
   claiming that any of these contracts are executable bootstrap features.

Runtime recording follows the existing recorded-effects contract. This chapter
adds no HTTP/TLS replay event IDs, secret capture policy or deterministic-network
claim. The compiler, networking library and complete release remain separate gates.
