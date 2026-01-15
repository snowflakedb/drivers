// Package wasip2 provides a WASI Preview 2 shim for wazero.
//
// This implements the wasi:sockets, wasi:io, and wasi:clocks interfaces
// that wstd (Rust WASM stdlib) requires for networking.
//
// The Go host handles actual TCP connections, giving full control over
// TLS, retries, and certificate validation.
package wasip2

import (
	"context"
	"crypto/tls"
	"fmt"
	"io"
	"net"
	"sync"
	"sync/atomic"
	"time"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
)

// Debug enables verbose logging
var Debug = false

func debugf(format string, args ...interface{}) {
	if Debug {
		fmt.Printf("[wasip2] "+format+"\n", args...)
	}
}

// Resource handle types
type handleType int

const (
	handleNetwork handleType = iota
	handleTCPSocket
	handleInputStream
	handleOutputStream
	handlePollable
	handleError
)

// Resource represents a WASI resource
type Resource struct {
	Type   handleType
	conn   net.Conn
	reader io.Reader
	writer io.Writer
	ready  bool
	err    error
}

// State holds all WASI Preview 2 state
type State struct {
	mu        sync.Mutex
	resources map[uint32]*Resource
	nextID    uint32
}

// NewState creates a new WASI Preview 2 state
func NewState() *State {
	return &State{
		resources: make(map[uint32]*Resource),
		nextID:    1,
	}
}

func (s *State) alloc(r *Resource) uint32 {
	s.mu.Lock()
	defer s.mu.Unlock()
	id := s.nextID
	s.nextID++
	s.resources[id] = r
	return id
}

func (s *State) get(id uint32) *Resource {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.resources[id]
}

func (s *State) free(id uint32) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if r, ok := s.resources[id]; ok {
		if r.conn != nil {
			r.conn.Close()
		}
		delete(s.resources, id)
	}
}

var globalState = NewState()
var connectTimeout = 30 * time.Second

// Instantiate registers all WASI Preview 2 modules with the runtime
func Instantiate(ctx context.Context, r wazero.Runtime) error {
	// wasi:sockets/instance-network@0.2.4
	if err := instantiateInstanceNetwork(ctx, r); err != nil {
		return err
	}

	// wasi:sockets/network@0.2.4
	if err := instantiateNetwork(ctx, r); err != nil {
		return err
	}

	// wasi:sockets/ip-name-lookup@0.2.4 (DNS resolution)
	if err := instantiateIPNameLookup(ctx, r); err != nil {
		return err
	}

	// wasi:sockets/tcp-create-socket@0.2.4
	if err := instantiateTCPCreateSocket(ctx, r); err != nil {
		return err
	}

	// wasi:sockets/tcp@0.2.4
	if err := instantiateTCP(ctx, r); err != nil {
		return err
	}

	// wasi:io/error@0.2.4
	if err := instantiateIOError(ctx, r); err != nil {
		return err
	}

	// wasi:io/poll@0.2.4
	if err := instantiatePoll(ctx, r); err != nil {
		return err
	}

	// wasi:io/streams@0.2.4
	if err := instantiateStreams(ctx, r); err != nil {
		return err
	}

	// wasi:clocks/monotonic-clock@0.2.4
	if err := instantiateMonotonicClock(ctx, r); err != nil {
		return err
	}

	// Custom DNS resolution (not WASI standard, but needed because std::net DNS doesn't work in WASM)
	if err := instantiateDNS(ctx, r); err != nil {
		return err
	}

	return nil
}

// === dns (custom module for DNS resolution) ===
// This provides DNS resolution since std::net::ToSocketAddrs doesn't work in WASM

func instantiateDNS(ctx context.Context, r wazero.Runtime) error {
	builder := r.NewHostModuleBuilder("dns")

	// resolve(hostname_ptr, hostname_len, result_ptr) -> i32
	// result_ptr will contain: [ok: u8, ip0: u8, ip1: u8, ip2: u8, ip3: u8]
	// Returns 0 on success, 1 on error
	builder.NewFunctionBuilder().
		WithGoModuleFunction(api.GoModuleFunc(func(ctx context.Context, m api.Module, stack []uint64) {
			hostnamePtr := uint32(stack[0])
			hostnameLen := uint32(stack[1])
			resultPtr := uint32(stack[2])

			hostnameBytes, ok := m.Memory().Read(hostnamePtr, hostnameLen)
			if !ok {
				stack[0] = 1 // error
				return
			}
			hostname := string(hostnameBytes)
			debugf("dns::resolve: %s", hostname)

			// Resolve DNS
			ips, err := net.LookupIP(hostname)
			if err != nil || len(ips) == 0 {
				debugf("  DNS lookup failed: %v", err)
				stack[0] = 1 // error
				return
			}

			// Find first IPv4 address
			var ipv4 net.IP
			for _, ip := range ips {
				if ip4 := ip.To4(); ip4 != nil {
					ipv4 = ip4
					break
				}
			}

			if ipv4 == nil {
				debugf("  no IPv4 address found")
				stack[0] = 1 // error
				return
			}

			debugf("  resolved to: %s", ipv4.String())

			// Store hostname -> IP mapping for TLS SNI lookup
			resolvedHosts.Store(hostname, ipv4)
			// Store reverse mapping: IP -> hostname for TLS SNI
			lastResolvedHostname.Store(ipv4.String(), hostname)

			// Write result
			m.Memory().WriteByte(resultPtr, 0) // ok
			m.Memory().WriteByte(resultPtr+1, ipv4[0])
			m.Memory().WriteByte(resultPtr+2, ipv4[1])
			m.Memory().WriteByte(resultPtr+3, ipv4[2])
			m.Memory().WriteByte(resultPtr+4, ipv4[3])

			stack[0] = 0 // success
		}), []api.ValueType{api.ValueTypeI32, api.ValueTypeI32, api.ValueTypeI32}, []api.ValueType{api.ValueTypeI32}).
		Export("resolve")

	_, err := builder.Instantiate(ctx)
	return err
}

// === wasi:sockets/ip-name-lookup@0.2.4 ===

func instantiateIPNameLookup(ctx context.Context, r wazero.Runtime) error {
	builder := r.NewHostModuleBuilder("wasi:sockets/ip-name-lookup@0.2.4")

	// resolve-addresses: (network, name) -> result<resolve-address-stream, error-code>
	builder.NewFunctionBuilder().
		WithGoModuleFunction(api.GoModuleFunc(func(ctx context.Context, m api.Module, stack []uint64) {
			// network := uint32(stack[0])
			namePtr := uint32(stack[1])
			nameLen := uint32(stack[2])
			resultPtr := uint32(stack[3])

			// Read hostname from WASM memory
			nameBytes, ok := m.Memory().Read(namePtr, nameLen)
			if !ok {
				m.Memory().WriteUint32Le(resultPtr, 1) // error
				return
			}
			hostname := string(nameBytes)
			debugf("resolve-addresses: %s", hostname)

			// Resolve DNS using Go
			ips, err := net.LookupIP(hostname)
			if err != nil || len(ips) == 0 {
				debugf("  DNS lookup failed: %v", err)
				m.Memory().WriteUint32Le(resultPtr, 1) // error
				return
			}

			// Find first IPv4 address
			var ipv4 net.IP
			for _, ip := range ips {
				if ip4 := ip.To4(); ip4 != nil {
					ipv4 = ip4
					break
				}
			}

			if ipv4 == nil {
				debugf("  no IPv4 address found")
				m.Memory().WriteUint32Le(resultPtr, 1) // error
				return
			}

			debugf("  resolved to: %s", ipv4.String())

			// Create a resolve stream handle that will return this IP
			streamHandle := globalState.alloc(&Resource{
				Type:  handlePollable,
				ready: true,
			})
			// Store the IP in a separate map for the stream to use
			resolvedIPs.Store(streamHandle, ipv4)

			// Write result: (ok=0, stream_handle)
			m.Memory().WriteUint32Le(resultPtr, 0)
			m.Memory().WriteUint32Le(resultPtr+4, streamHandle)
		}), []api.ValueType{api.ValueTypeI32, api.ValueTypeI32, api.ValueTypeI32, api.ValueTypeI32}, []api.ValueType{}).
		Export("resolve-addresses")

	// [resource-drop]resolve-address-stream
	builder.NewFunctionBuilder().
		WithGoFunction(api.GoFunc(func(ctx context.Context, stack []uint64) {
			handle := uint32(stack[0])
			debugf("resolve-address-stream drop: %d", handle)
			resolvedIPs.Delete(handle)
			globalState.free(handle)
		}), []api.ValueType{api.ValueTypeI32}, []api.ValueType{}).
		Export("[resource-drop]resolve-address-stream")

	// [method]resolve-address-stream.resolve-next-address
	builder.NewFunctionBuilder().
		WithGoModuleFunction(api.GoModuleFunc(func(ctx context.Context, m api.Module, stack []uint64) {
			handle := uint32(stack[0])
			resultPtr := uint32(stack[1])

			debugf("resolve-next-address: %d", handle)

			ipVal, ok := resolvedIPs.Load(handle)
			if !ok {
				// No more addresses
				m.Memory().WriteUint32Le(resultPtr, 2) // closed/done
				return
			}

			ip := ipVal.(net.IP)
			resolvedIPs.Delete(handle) // Only return once

			// Write result: (ok=0, ip-address)
			// ip-address is a variant: (tag=0 for IPv4, then 4 bytes)
			m.Memory().WriteUint32Le(resultPtr, 0)   // ok
			m.Memory().WriteUint32Le(resultPtr+4, 0) // IPv4 tag
			m.Memory().WriteByte(resultPtr+8, ip[0])
			m.Memory().WriteByte(resultPtr+9, ip[1])
			m.Memory().WriteByte(resultPtr+10, ip[2])
			m.Memory().WriteByte(resultPtr+11, ip[3])

			debugf("  returning IP: %s", ip.String())
		}), []api.ValueType{api.ValueTypeI32, api.ValueTypeI32}, []api.ValueType{}).
		Export("[method]resolve-address-stream.resolve-next-address")

	// [method]resolve-address-stream.subscribe
	builder.NewFunctionBuilder().
		WithGoFunction(api.GoFunc(func(ctx context.Context, stack []uint64) {
			handle := uint32(stack[0])
			debugf("resolve-address-stream.subscribe: %d", handle)
			stack[0] = uint64(globalState.alloc(&Resource{Type: handlePollable, ready: true}))
		}), []api.ValueType{api.ValueTypeI32}, []api.ValueType{api.ValueTypeI32}).
		Export("[method]resolve-address-stream.subscribe")

	_, err := builder.Instantiate(ctx)
	return err
}

// resolvedIPs stores resolved IP addresses for each stream handle
var resolvedIPs sync.Map

// socketPollables maps pollable handles to socket handles for connection tracking
var socketPollables sync.Map

// resolvedHosts maps hostname -> IP for TLS SNI lookup
var resolvedHosts sync.Map

// streamPollables maps pollable handles to stream handles
var streamPollables sync.Map

// connections maps socket handles to established net.Conn
var connections sync.Map

// === wasi:sockets/instance-network@0.2.4 ===

func instantiateInstanceNetwork(ctx context.Context, r wazero.Runtime) error {
	_, err := r.NewHostModuleBuilder("wasi:sockets/instance-network@0.2.4").
		NewFunctionBuilder().
		WithGoFunction(api.GoFunc(func(ctx context.Context, stack []uint64) {
			debugf("instance-network")
			stack[0] = uint64(globalState.alloc(&Resource{Type: handleNetwork}))
		}), []api.ValueType{}, []api.ValueType{api.ValueTypeI32}).
		Export("instance-network").
		Instantiate(ctx)
	return err
}

// === wasi:sockets/network@0.2.4 ===

func instantiateNetwork(ctx context.Context, r wazero.Runtime) error {
	_, err := r.NewHostModuleBuilder("wasi:sockets/network@0.2.4").
		NewFunctionBuilder().
		WithGoFunction(api.GoFunc(func(ctx context.Context, stack []uint64) {
			handle := uint32(stack[0])
			debugf("network drop: %d", handle)
			globalState.free(handle)
		}), []api.ValueType{api.ValueTypeI32}, []api.ValueType{}).
		Export("[resource-drop]network").
		Instantiate(ctx)
	return err
}

// === wasi:sockets/tcp-create-socket@0.2.4 ===

func instantiateTCPCreateSocket(ctx context.Context, r wazero.Runtime) error {
	_, err := r.NewHostModuleBuilder("wasi:sockets/tcp-create-socket@0.2.4").
		NewFunctionBuilder().
		WithGoModuleFunction(api.GoModuleFunc(func(ctx context.Context, m api.Module, stack []uint64) {
			addressFamily := uint32(stack[0])
			resultPtr := uint32(stack[1])
			debugf("create-tcp-socket: family=%d", addressFamily)
			socketHandle := globalState.alloc(&Resource{Type: handleTCPSocket})
			// Write result: (ok=0, socket_handle)
			m.Memory().WriteUint32Le(resultPtr, 0)
			m.Memory().WriteUint32Le(resultPtr+4, socketHandle)
		}), []api.ValueType{api.ValueTypeI32, api.ValueTypeI32}, []api.ValueType{}).
		Export("create-tcp-socket").
		Instantiate(ctx)
	return err
}

// === wasi:sockets/tcp@0.2.4 ===

// Pending connection state
var pendingConns sync.Map         // socketHandle -> *pendingConn
var lastResolvedHostname sync.Map // IP string -> hostname (for TLS SNI)

type pendingConn struct {
	addr string
	host string // For TLS SNI
	port uint16
	conn net.Conn
	err  error
	done atomic.Bool
}

func instantiateTCP(ctx context.Context, r wazero.Runtime) error {
	builder := r.NewHostModuleBuilder("wasi:sockets/tcp@0.2.4")

	// [resource-drop]tcp-socket
	builder.NewFunctionBuilder().
		WithGoFunction(api.GoFunc(func(ctx context.Context, stack []uint64) {
			handle := uint32(stack[0])
			debugf("tcp-socket drop: %d", handle)
			pendingConns.Delete(handle)
			globalState.free(handle)
		}), []api.ValueType{api.ValueTypeI32}, []api.ValueType{}).
		Export("[resource-drop]tcp-socket")

	// [method]tcp-socket.start-connect
	// 15 i32 params for IPv4: socket, network, ip_tag, port, ip0, ip1, ip2, ip3, ...padding..., result_ptr
	// WASI component model flattens IpSocketAddress variant
	builder.NewFunctionBuilder().
		WithGoModuleFunction(api.GoModuleFunc(func(ctx context.Context, m api.Module, stack []uint64) {
			socketHandle := uint32(stack[0])
			// network := uint32(stack[1])
			ipTag := uint32(stack[2])
			resultPtr := uint32(stack[14]) // Last param

			debugf("start-connect: socket=%d ipTag=%d", socketHandle, ipTag)
			debugf("  raw params: %v", stack[:15])

			var ip net.IP
			var port uint16

			if ipTag == 0 {
				// IPv4 IpSocketAddress layout: (tag, port, (ip0, ip1, ip2, ip3))
				port = uint16(stack[3])
				ip = net.IPv4(byte(stack[4]), byte(stack[5]), byte(stack[6]), byte(stack[7]))
			} else {
				// IPv6: (tag, port, flowinfo, (8 u16s for IP), scope_id)
				port = uint16(stack[3])
				// flowinfo = stack[4]
				ip = make(net.IP, 16)
				for i := 0; i < 8; i++ {
					val := uint16(stack[5+i])
					ip[i*2] = byte(val >> 8)
					ip[i*2+1] = byte(val)
				}
				// scope_id = stack[13]
			}

			addr := fmt.Sprintf("%s:%d", ip.String(), port)
			debugf("  connecting to: %s", addr)

			// Start async connect - Go handles TLS transparently for port 443
			pc := &pendingConn{addr: addr, host: ip.String(), port: port}
			pendingConns.Store(socketHandle, pc)

			// Look up the hostname from DNS resolution (stored earlier)
			hostname := ""
			if h, ok := lastResolvedHostname.Load(ip.String()); ok {
				hostname = h.(string)
			}

			go func() {
				conn, err := net.DialTimeout("tcp", addr, connectTimeout)
				if err != nil {
					pc.err = err
					pc.done.Store(true)
					debugf("  connect failed: %s err=%v", addr, err)
					return
				}

				// Upgrade to TLS for port 443
				if port == 443 {
					debugf("  upgrading to TLS for %s (hostname=%s)", addr, hostname)
					tlsConfig := &tls.Config{
						ServerName: hostname, // Use original hostname for SNI
					}
					if hostname == "" {
						// Fallback: try to use the IP, but this will likely fail certificate validation
						tlsConfig.ServerName = ip.String()
						debugf("  warning: no hostname for SNI, using IP %s", ip.String())
					}
					tlsConn := tls.Client(conn, tlsConfig)
					if err := tlsConn.Handshake(); err != nil {
						debugf("  TLS handshake failed: %v", err)
						pc.err = fmt.Errorf("TLS handshake failed: %w", err)
						pc.done.Store(true)
						return
					}
					debugf("  TLS handshake completed")
					conn = tlsConn
				}

				pc.conn = conn
				pc.done.Store(true)
				debugf("  connect completed: %s", addr)
			}()

			// Return pollable handle for waiting
			pollableHandle := globalState.alloc(&Resource{
				Type: handlePollable,
			})

			// Write result: (ok=0, pollable_handle)
			m.Memory().WriteUint32Le(resultPtr, 0) // ok
			m.Memory().WriteUint32Le(resultPtr+4, pollableHandle)
		}), []api.ValueType{
			api.ValueTypeI32, api.ValueTypeI32, api.ValueTypeI32, api.ValueTypeI32,
			api.ValueTypeI32, api.ValueTypeI32, api.ValueTypeI32, api.ValueTypeI32,
			api.ValueTypeI32, api.ValueTypeI32, api.ValueTypeI32, api.ValueTypeI32,
			api.ValueTypeI32, api.ValueTypeI32, api.ValueTypeI32,
		}, []api.ValueType{}).
		Export("[method]tcp-socket.start-connect")

	// [method]tcp-socket.finish-connect - 2 params: socket, result_ptr
	builder.NewFunctionBuilder().
		WithGoModuleFunction(api.GoModuleFunc(func(ctx context.Context, m api.Module, stack []uint64) {
			socketHandle := uint32(stack[0])
			resultPtr := uint32(stack[1])

			debugf("finish-connect: socket=%d", socketHandle)

			pcVal, ok := pendingConns.Load(socketHandle)
			if !ok {
				debugf("  no pending connection found!")
				m.Memory().WriteUint32Le(resultPtr, 1) // error
				return
			}
			pc := pcVal.(*pendingConn)

			debugf("  pending conn: addr=%s done=%v", pc.addr, pc.done.Load())

			if !pc.done.Load() {
				debugf("  not done yet, would-block")
				m.Memory().WriteUint32Le(resultPtr, 1) // error: would-block
				return
			}

			if pc.err != nil {
				debugf("  connect error: %v", pc.err)
				m.Memory().WriteUint32Le(resultPtr, 1) // error
				return
			}

			resource := globalState.get(socketHandle)
			if resource != nil {
				resource.conn = pc.conn
			}

			// Store in connections map for polling
			connections.Store(socketHandle, pc.conn)

			inputHandle := globalState.alloc(&Resource{
				Type:   handleInputStream,
				conn:   pc.conn,
				reader: pc.conn,
			})
			outputHandle := globalState.alloc(&Resource{
				Type:   handleOutputStream,
				conn:   pc.conn,
				writer: pc.conn,
			})

			debugf("  connected! input=%d output=%d", inputHandle, outputHandle)

			// Write result: (ok=0, input_stream, output_stream)
			m.Memory().WriteUint32Le(resultPtr, 0) // ok
			m.Memory().WriteUint32Le(resultPtr+4, inputHandle)
			m.Memory().WriteUint32Le(resultPtr+8, outputHandle)

			pendingConns.Delete(socketHandle)
		}), []api.ValueType{api.ValueTypeI32, api.ValueTypeI32}, []api.ValueType{}).
		Export("[method]tcp-socket.finish-connect")

	// [method]tcp-socket.subscribe - 1 param, 1 result
	// Returns a pollable that becomes ready when the socket operation completes
	builder.NewFunctionBuilder().
		WithGoFunction(api.GoFunc(func(ctx context.Context, stack []uint64) {
			socketHandle := uint32(stack[0])
			debugf("tcp-socket.subscribe: socket=%d", socketHandle)

			// Create a pollable that tracks this socket's connection state
			pollableHandle := globalState.alloc(&Resource{Type: handlePollable})

			// Store the socket handle so we can check if the connection is done
			socketPollables.Store(pollableHandle, socketHandle)

			stack[0] = uint64(pollableHandle)
		}), []api.ValueType{api.ValueTypeI32}, []api.ValueType{api.ValueTypeI32}).
		Export("[method]tcp-socket.subscribe")

	// [method]tcp-socket.shutdown - 3 params: socket, shutdown_type, result_ptr
	builder.NewFunctionBuilder().
		WithGoModuleFunction(api.GoModuleFunc(func(ctx context.Context, m api.Module, stack []uint64) {
			socketHandle := uint32(stack[0])
			shutdownType := uint32(stack[1])
			resultPtr := uint32(stack[2])

			debugf("tcp-socket.shutdown: socket=%d type=%d", socketHandle, shutdownType)
			resource := globalState.get(socketHandle)
			if resource != nil && resource.conn != nil {
				resource.conn.Close()
			}
			m.Memory().WriteUint32Le(resultPtr, 0) // ok
		}), []api.ValueType{api.ValueTypeI32, api.ValueTypeI32, api.ValueTypeI32}, []api.ValueType{}).
		Export("[method]tcp-socket.shutdown")

	_, err := builder.Instantiate(ctx)
	return err
}

// === wasi:io/error@0.2.4 ===

func instantiateIOError(ctx context.Context, r wazero.Runtime) error {
	builder := r.NewHostModuleBuilder("wasi:io/error@0.2.4")

	builder.NewFunctionBuilder().
		WithGoFunction(api.GoFunc(func(ctx context.Context, stack []uint64) {
			handle := uint32(stack[0])
			debugf("error drop: %d", handle)
			globalState.free(handle)
		}), []api.ValueType{api.ValueTypeI32}, []api.ValueType{}).
		Export("[resource-drop]error")

	builder.NewFunctionBuilder().
		WithGoModuleFunction(api.GoModuleFunc(func(ctx context.Context, m api.Module, stack []uint64) {
			handle := uint32(stack[0])
			resultPtr := uint32(stack[1])
			debugf("error.to-debug-string: %d", handle)
			resource := globalState.get(handle)
			errStr := "unknown error"
			if resource != nil && resource.err != nil {
				errStr = resource.err.Error()
			}
			// Write string length
			strBytes := []byte(errStr)
			m.Memory().WriteUint32Le(resultPtr, uint32(len(strBytes)))
		}), []api.ValueType{api.ValueTypeI32, api.ValueTypeI32}, []api.ValueType{}).
		Export("[method]error.to-debug-string")

	_, err := builder.Instantiate(ctx)
	return err
}

// === wasi:io/poll@0.2.4 ===

func instantiatePoll(ctx context.Context, r wazero.Runtime) error {
	builder := r.NewHostModuleBuilder("wasi:io/poll@0.2.4")

	builder.NewFunctionBuilder().
		WithGoFunction(api.GoFunc(func(ctx context.Context, stack []uint64) {
			handle := uint32(stack[0])
			debugf("pollable drop: %d", handle)
			globalState.free(handle)
		}), []api.ValueType{api.ValueTypeI32}, []api.ValueType{}).
		Export("[resource-drop]pollable")

	// [method]pollable.ready - 1 param, 1 result (bool as i32)
	builder.NewFunctionBuilder().
		WithGoFunction(api.GoFunc(func(ctx context.Context, stack []uint64) {
			handle := uint32(stack[0])
			debugf("pollable.ready: handle=%d", handle)

			resource := globalState.get(handle)
			if resource != nil && resource.ready {
				debugf("  ready (resource.ready=true)")
				stack[0] = 1 // true
				return
			}

			// Check if this pollable is for a socket connection
			if socketHandle, ok := socketPollables.Load(handle); ok {
				if pcVal, ok := pendingConns.Load(socketHandle); ok {
					pc := pcVal.(*pendingConn)
					if pc.done.Load() {
						debugf("  ready (socket %d connection done)", socketHandle)
						stack[0] = 1 // true
						return
					}
				}
			}

			debugf("  not ready")
			stack[0] = 0 // not ready
		}), []api.ValueType{api.ValueTypeI32}, []api.ValueType{api.ValueTypeI32}).
		Export("[method]pollable.ready")

	// poll - list<pollable> -> list<u32>
	// params: list_ptr, list_len, result_ptr
	// Returns indices of ready pollables
	builder.NewFunctionBuilder().
		WithGoModuleFunction(api.GoModuleFunc(func(ctx context.Context, m api.Module, stack []uint64) {
			listPtr := uint32(stack[0])
			listLen := uint32(stack[1])
			resultPtr := uint32(stack[2])

			debugf("poll: list_ptr=%d list_len=%d", listPtr, listLen)

			// Read pollable handles from the list
			handles := make([]uint32, listLen)
			for i := uint32(0); i < listLen; i++ {
				handles[i], _ = m.Memory().ReadUint32Le(listPtr + i*4)
			}
			debugf("poll: handles=%v", handles)

			// Wait for at least one pollable to be ready
			for attempt := 0; attempt < 1000; attempt++ { // Max 10 seconds
				var readyIndices []uint32

				for i, handle := range handles {
					isReady := false

					// Check if directly ready
					resource := globalState.get(handle)
					if resource != nil && resource.ready {
						isReady = true
					}

					// Check if this is a socket pollable
					if !isReady {
						if socketHandle, ok := socketPollables.Load(handle); ok {
							if pcVal, ok := pendingConns.Load(socketHandle); ok {
								pc := pcVal.(*pendingConn)
								if pc.done.Load() {
									isReady = true
								}
							}
						}
					}

					// Check if this is a stream pollable
					if !isReady {
						if streamHandle, ok := streamPollables.Load(handle); ok {
							if conn, ok := connections.Load(streamHandle); ok {
								// For read pollables, we consider them ready if there's data or connection closed
								// For simplicity, mark as ready
								_ = conn
								isReady = true
							}
						}
					}

					if isReady {
						readyIndices = append(readyIndices, uint32(i))
					}
				}

				if len(readyIndices) > 0 {
					debugf("poll: %d pollables ready, indices=%v", len(readyIndices), readyIndices)
					// WASI component model: list<u32> is returned as (data_ptr, length)
					// We need to allocate space for the data and write the pointer
					// For simplicity, write data inline at resultPtr+8 and point to it
					dataPtr := resultPtr + 8
					m.Memory().WriteUint32Le(resultPtr, dataPtr)                     // ptr to data
					m.Memory().WriteUint32Le(resultPtr+4, uint32(len(readyIndices))) // length
					for i, idx := range readyIndices {
						m.Memory().WriteUint32Le(dataPtr+uint32(i)*4, idx)
					}
					return
				}

				time.Sleep(10 * time.Millisecond)
			}

			// Timeout - return empty list (ptr=0, len=0)
			debugf("poll: timeout")
			m.Memory().WriteUint32Le(resultPtr, 0)   // ptr (null)
			m.Memory().WriteUint32Le(resultPtr+4, 0) // length
		}), []api.ValueType{api.ValueTypeI32, api.ValueTypeI32, api.ValueTypeI32}, []api.ValueType{}).
		Export("poll")

	_, err := builder.Instantiate(ctx)
	return err
}

// === wasi:io/streams@0.2.4 ===

func instantiateStreams(ctx context.Context, r wazero.Runtime) error {
	builder := r.NewHostModuleBuilder("wasi:io/streams@0.2.4")

	// Input stream
	builder.NewFunctionBuilder().
		WithGoFunction(api.GoFunc(func(ctx context.Context, stack []uint64) {
			handle := uint32(stack[0])
			debugf("input-stream drop: %d", handle)
			globalState.free(handle)
		}), []api.ValueType{api.ValueTypeI32}, []api.ValueType{}).
		Export("[resource-drop]input-stream")

	// [method]input-stream.read - params: handle, max_len (u64), result_ptr
	// For component model, we need to allocate memory in WASM for the returned data
	builder.NewFunctionBuilder().
		WithGoModuleFunction(api.GoModuleFunc(func(ctx context.Context, m api.Module, stack []uint64) {
			handle := uint32(stack[0])
			maxLen := stack[1]
			resultPtr := uint32(stack[2])

			debugf("input-stream.read: handle=%d maxLen=%d", handle, maxLen)

			resource := globalState.get(handle)
			if resource == nil || resource.reader == nil {
				m.Memory().WriteUint32Le(resultPtr, 1) // error
				return
			}

			// Limit read size
			if maxLen > 65536 {
				maxLen = 65536
			}

			buf := make([]byte, maxLen)
			n, err := resource.reader.Read(buf)

			if err != nil && err != io.EOF {
				debugf("  read error: %v", err)
				m.Memory().WriteUint32Le(resultPtr, 1) // error
				return
			}

			if n == 0 && err == io.EOF {
				debugf("  EOF")
				m.Memory().WriteUint32Le(resultPtr, 2) // closed
				return
			}

			debugf("  read %d bytes", n)

			// Allocate memory in WASM for the data using the module's allocator
			allocFn := m.ExportedFunction("cabi_realloc")
			if allocFn == nil {
				debugf("  no cabi_realloc found!")
				m.Memory().WriteUint32Le(resultPtr, 1) // error
				return
			}

			// cabi_realloc(old_ptr, old_size, align, new_size) -> ptr
			results, err := allocFn.Call(ctx, 0, 0, 1, uint64(n))
			if err != nil || len(results) == 0 {
				debugf("  cabi_realloc failed: %v", err)
				m.Memory().WriteUint32Le(resultPtr, 1) // error
				return
			}
			dataPtr := uint32(results[0])
			debugf("  allocated at: %d", dataPtr)

			// Write data to WASM memory
			if !m.Memory().Write(dataPtr, buf[:n]) {
				debugf("  memory write failed!")
				m.Memory().WriteUint32Le(resultPtr, 1) // error
				return
			}

			// Write result: result<list<u8>, stream-error>
			// For ok variant: (ok_tag=0, list<u8> as (ptr, len))
			m.Memory().WriteUint32Le(resultPtr, 0)           // ok tag
			m.Memory().WriteUint32Le(resultPtr+4, dataPtr)   // ptr to data
			m.Memory().WriteUint32Le(resultPtr+8, uint32(n)) // length
		}), []api.ValueType{api.ValueTypeI32, api.ValueTypeI64, api.ValueTypeI32}, []api.ValueType{}).
		Export("[method]input-stream.read")

	builder.NewFunctionBuilder().
		WithGoFunction(api.GoFunc(func(ctx context.Context, stack []uint64) {
			handle := uint32(stack[0])
			debugf("input-stream.subscribe: %d", handle)
			stack[0] = uint64(globalState.alloc(&Resource{Type: handlePollable, ready: true}))
		}), []api.ValueType{api.ValueTypeI32}, []api.ValueType{api.ValueTypeI32}).
		Export("[method]input-stream.subscribe")

	// Output stream
	builder.NewFunctionBuilder().
		WithGoFunction(api.GoFunc(func(ctx context.Context, stack []uint64) {
			handle := uint32(stack[0])
			debugf("output-stream drop: %d", handle)
			globalState.free(handle)
		}), []api.ValueType{api.ValueTypeI32}, []api.ValueType{}).
		Export("[resource-drop]output-stream")

	// [method]output-stream.check-write - params: handle, result_ptr
	builder.NewFunctionBuilder().
		WithGoModuleFunction(api.GoModuleFunc(func(ctx context.Context, m api.Module, stack []uint64) {
			handle := uint32(stack[0])
			resultPtr := uint32(stack[1])
			debugf("output-stream.check-write: %d", handle)
			// Return large available capacity: (ok=0, u64 capacity)
			m.Memory().WriteUint32Le(resultPtr, 0)           // ok
			m.Memory().WriteUint64Le(resultPtr+4, 1024*1024) // 1MB
		}), []api.ValueType{api.ValueTypeI32, api.ValueTypeI32}, []api.ValueType{}).
		Export("[method]output-stream.check-write")

	// [method]output-stream.write - params: handle, data_ptr, data_len, result_ptr
	builder.NewFunctionBuilder().
		WithGoModuleFunction(api.GoModuleFunc(func(ctx context.Context, m api.Module, stack []uint64) {
			handle := uint32(stack[0])
			dataPtr := uint32(stack[1])
			dataLen := uint32(stack[2])
			resultPtr := uint32(stack[3])

			debugf("output-stream.write: handle=%d len=%d", handle, dataLen)

			resource := globalState.get(handle)
			if resource == nil || resource.writer == nil {
				m.Memory().WriteUint32Le(resultPtr, 1) // error
				return
			}

			data, ok := m.Memory().Read(dataPtr, dataLen)
			if !ok {
				m.Memory().WriteUint32Le(resultPtr, 1) // error
				return
			}

			debugf("  writing first 16 bytes: %x", data[:min(16, len(data))])

			n, err := resource.writer.Write(data)
			if err != nil {
				debugf("  write error: %v", err)
				m.Memory().WriteUint32Le(resultPtr, 1) // error
				return
			}

			debugf("  wrote %d bytes", n)
			m.Memory().WriteUint32Le(resultPtr, 0) // ok
		}), []api.ValueType{api.ValueTypeI32, api.ValueTypeI32, api.ValueTypeI32, api.ValueTypeI32}, []api.ValueType{}).
		Export("[method]output-stream.write")

	builder.NewFunctionBuilder().
		WithGoFunction(api.GoFunc(func(ctx context.Context, stack []uint64) {
			handle := uint32(stack[0])
			debugf("output-stream.subscribe: %d", handle)
			stack[0] = uint64(globalState.alloc(&Resource{Type: handlePollable, ready: true}))
		}), []api.ValueType{api.ValueTypeI32}, []api.ValueType{api.ValueTypeI32}).
		Export("[method]output-stream.subscribe")

	_, err := builder.Instantiate(ctx)
	return err
}

// === wasi:clocks/monotonic-clock@0.2.4 ===

func instantiateMonotonicClock(ctx context.Context, r wazero.Runtime) error {
	builder := r.NewHostModuleBuilder("wasi:clocks/monotonic-clock@0.2.4")

	// subscribe-duration: u64 -> pollable
	builder.NewFunctionBuilder().
		WithGoFunction(api.GoFunc(func(ctx context.Context, stack []uint64) {
			nanoseconds := stack[0]
			debugf("subscribe-duration: %d ns", nanoseconds)
			pollable := globalState.alloc(&Resource{Type: handlePollable})
			go func() {
				time.Sleep(time.Duration(nanoseconds))
				if r := globalState.get(pollable); r != nil {
					r.ready = true
				}
			}()
			stack[0] = uint64(pollable)
		}), []api.ValueType{api.ValueTypeI64}, []api.ValueType{api.ValueTypeI32}).
		Export("subscribe-duration")

	_, err := builder.Instantiate(ctx)
	return err
}
