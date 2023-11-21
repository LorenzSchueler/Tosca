package st

import (
	"fmt"
	"math"
	"slices"

	. "github.com/Fantom-foundation/Tosca/go/ct/common"
	"golang.org/x/crypto/sha3"
)

// Memory represents the EVM's execution memory.
type Memory struct {
	mem []byte
}

// NewMemory creates a new memory filled with the given values.
func NewMemory(data ...byte) *Memory {
	return &Memory{data}
}

// Clone creates an independent copy of the memory.
func (m *Memory) Clone() *Memory {
	return &Memory{slices.Clone(m.mem)}
}

// Size returns the memory size in bytes.
func (m *Memory) Size() int {
	return len(m.mem)
}

// Set sets the memory content to a copy of the given slice.
func (m *Memory) Set(data []byte) {
	m.mem = slices.Clone(data)
}

// Append appends the given data to the memory content.
func (m *Memory) Append(data []byte) {
	m.mem = append(m.mem, data...)
}

// Read retrieves size bytes from memory starting at offset. Memory is
// automatically grown and zero-initialized.
func (m *Memory) Read(offset, size uint64) []byte {
	if size == 0 {
		return nil
	}
	m.Grow(offset, size)
	return m.mem[offset : offset+size]
}

// Write stores the given data in memory starting at offset. Memory is
// automatically grown and zero-initialized.
func (m *Memory) Write(data []byte, offset uint64) {
	if len(data) == 0 {
		return
	}
	m.Grow(offset, uint64(len(data)))
	copy(m.mem[offset:], data)
}

// Grow increases the memory allocation to accommodate for offset + size bytes.
// No allocation happens when size is zero. Memory is grown in 32 byte steps
func (m *Memory) Grow(offset, size uint64) {
	if size == 0 {
		return
	}
	newSize := offset + size
	if newSize > uint64(m.Size()) {
		newSize = ((newSize + 31) / 32) * 32
		m.mem = append(m.mem, make([]byte, newSize-uint64(m.Size()))...)
	}
}

// ExpansionCosts calculates the expansion costs for the given offset and size.
// It does not grow memory. It also returns offset and size converted to uint64.
func (m *Memory) ExpansionCosts(offset_u256, size_u256 U256) (memCost, offset, size uint64) {
	if !size_u256.IsUint64() || (!offset_u256.IsUint64() && !size_u256.IsZero()) {
		return math.MaxUint64, 0, 0
	}
	offset = offset_u256.Uint64()
	size = size_u256.Uint64()
	if size == 0 {
		memCost = 0
		return
	}
	newSize := offset + size
	if newSize <= uint64(m.Size()) {
		memCost = 0
		return
	}
	calcMemoryCost := func(size uint64) uint64 {
		memorySizeWord := (size + 31) / 32
		return (memorySizeWord*memorySizeWord)/512 + (3 * memorySizeWord)
	}
	memCost = calcMemoryCost(newSize) - calcMemoryCost(uint64(m.Size()))
	return
}

// Hash calculates the hash of the given memory span. Memory is grown automatically.
func (m *Memory) Hash(offset, size uint64) (hash [32]byte) {
	m.Grow(offset, size)

	hasher := sha3.NewLegacyKeccak256()

	// slice[offset:_] panics if offset is out-of-bounds, even when the
	// resulting slice would be empty.
	if size > 0 {
		hasher.Write(m.mem[offset : offset+size])
	}

	copy(hash[:], hasher.Sum(nil)[:])
	return
}

// Eq returns true if the two memory instances are equal.
func (a *Memory) Eq(b *Memory) bool {
	return slices.Equal(a.mem, b.mem)
}

// Diff returns a list of differences between the two memory instance.
func (a *Memory) Diff(b *Memory) (res []string) {
	if a.Size() != b.Size() {
		res = append(res, fmt.Sprintf("Different memory size: %v vs %v", a.Size(), b.Size()))
		return
	}
	for i := 0; i < a.Size(); i++ {
		if aValue, bValue := a.mem[i], b.mem[i]; aValue != bValue {
			res = append(res, fmt.Sprintf("Different memory value at offset %d: %v vs %v", i, aValue, bValue))
		}
	}
	return
}
