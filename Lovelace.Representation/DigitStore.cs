using System.Text;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

[assembly: InternalsVisibleTo("Lovelace.Representation.Tests")]

namespace Lovelace.Representation;

/// <summary>
/// BCD digit storage: two decimal digits packed per byte.
/// High nibble (bits 7–4) holds the even-indexed digit;
/// low nibble (bits 3–0) holds the odd-indexed digit.
/// Sentinel values: 0x0C = slot available, 0x0F = slot freed.
/// </summary>
public class DigitStore
{
    // -------------------------------------------------------------------------
    // Backing fields
    // -------------------------------------------------------------------------
    private readonly List<byte> _bytes;
    private long _digitCount;
    private bool _isZero;

    /// <summary>Monitor used for all read/write synchronisation on this instance.</summary>
    private readonly object _syncRoot = new();

    // Monotonically increasing ID assigned at construction; used by CopyDigitsFrom to
    // establish a canonical lock order and thereby prevent ABBA deadlocks when two
    // threads simultaneously copy from each other.
    private static long _idCounter;
    private readonly long _id = Interlocked.Increment(ref _idCounter);

    // -------------------------------------------------------------------------
    // Constructors
    // -------------------------------------------------------------------------

    /// <summary>Default constructor — produces an empty zero store.</summary>
    public DigitStore()
    {
        _bytes = [];
        Initialize();
    }

    /// <summary>Copy constructor — deep-copies <paramref name="other"/>.</summary>
    public DigitStore(DigitStore other)
    {
        _bytes = [];
        // Snapshot other's state atomically so we get a consistent view.
        lock (other._syncRoot)
        {
            _digitCount = other._digitCount;
            _isZero = other._isZero;
            if (!other._isZero)
                _bytes.AddRange(other._bytes);
        }
    }

    // -------------------------------------------------------------------------
    // Public properties
    // -------------------------------------------------------------------------

    /// <summary>Number of bytes in the backing store.</summary>
    public long ByteCount => _bytes.Count;

    /// <summary>Number of logical decimal digits stored.</summary>
    public long DigitCount => _digitCount;

    /// <summary>True when the number represented is zero.</summary>
    public bool IsZero => _isZero;

    /// <summary>
    /// Sets <see cref="DigitCount"/> to <paramref name="value"/> under <c>_syncRoot</c>.
    /// Use instead of the former <c>internal set</c> accessor to ensure the write is
    /// covered by the same monitor lock used by all other mutating methods.
    /// </summary>
    internal void SetDigitCount(long value)
    {
        lock (_syncRoot)
        {
            _digitCount = value;
        }
    }

    /// <summary>
    /// Sets <see cref="IsZero"/> to <paramref name="value"/> under <c>_syncRoot</c>.
    /// Use instead of the former <c>internal set</c> accessor to ensure the write is
    /// covered by the same monitor lock used by all other mutating methods.
    /// </summary>
    internal void SetIsZero(bool value)
    {
        lock (_syncRoot)
        {
            _isZero = value;
        }
    }

    // -------------------------------------------------------------------------
    // Public digit access
    // -------------------------------------------------------------------------

    /// <summary>
    /// Returns the decimal digit (0–9) at <paramref name="position"/>.
    /// Position 0 is the least-significant digit.
    /// Returns 0 when <see cref="IsZero"/> is true or position is out of range.
    /// </summary>
    public byte GetDigit(long position)
    {
        lock (_syncRoot)
        {
            if (_isZero)
                return 0;

            if (position >= 0 && position < _digitCount)
            {
                GetBitwise(position / 2, out byte high, out byte low);
                return (position % 2 == 0) ? high : low;
            }

            return 0;
        }
    }

    /// <summary>
    /// Writes a decimal digit (0–9) at <paramref name="position"/>.
    /// Position must be ≤ <see cref="DigitCount"/> (sequential writes only).
    /// </summary>
    /// <exception cref="ArgumentOutOfRangeException">
    /// Thrown when <paramref name="position"/> is negative or greater than <see cref="DigitCount"/>.
    /// </exception>
    public void SetDigit(long position, byte digit)
    {
        lock (_syncRoot)
        {
            if (position < 0 || position > _digitCount)
                throw new ArgumentOutOfRangeException(nameof(position),
                    $"position {position} is out of range (DigitCount={_digitCount}).");

            byte high, low;

            if (position / 2 < ByteCount)
            {
                // Byte slot already exists — read-modify-write
                GetBitwise(position / 2, out high, out low);
                if (position % 2 == 0)
                    high = digit;
                else
                    low = digit;
            }
            else
            {
                // New byte slot — initialize sentinel low nibble
                high = digit;
                low = 0x0F;
            }

            if (position >= _digitCount)
                _digitCount++;

            if (position == 0)
                _isZero = false;

            SetBitwise(position / 2, high, low);
        }
    }

    // -------------------------------------------------------------------------
    // Internal BCD infrastructure
    // -------------------------------------------------------------------------

    /// <summary>
    /// Splits the byte at <paramref name="pos"/> into its high and low nibbles.
    /// No-op (silent) when <paramref name="pos"/> is out of range.
    /// </summary>
    internal void GetBitwise(long pos, out byte high, out byte low)
    {
        if (pos >= 0 && pos < ByteCount)
        {
            byte coded = _bytes[(int)pos];
            high = (byte)((coded & 0xF0) >> 4);
            low = (byte)(coded & 0x0F);
        }
        else
        {
            high = 0;
            low = 0;
        }
    }

    /// <summary>
    /// Packs <paramref name="high"/> and <paramref name="low"/> into the byte at
    /// <paramref name="pos"/>. Calls <see cref="GrowDigits"/> when
    /// <paramref name="pos"/> equals <see cref="ByteCount"/>.
    /// Silent no-op when position is beyond ByteCount + 1.
    /// </summary>
    internal void SetBitwise(long pos, byte high, byte low)
    {
        lock (_syncRoot)
        {
            long size = ByteCount;

            if (pos >= 0 && pos <= size)
            {
                byte packed = (byte)((high << 4) | (low & 0x0F));

                if (pos == size)
                    GrowDigits();

                _bytes[(int)pos] = packed;
            }
            // Silently ignore out-of-range positions
        }
    }

    /// <summary>Appends a sentinel byte (0x0C) to grow the backing store by one byte.</summary>
    internal void GrowDigits()
    {
        lock (_syncRoot)
        {
            _bytes.Add(0x0C);
        }
    }

    /// <summary>
    /// Removes the most-significant digit slot.
    /// When <see cref="DigitCount"/> is odd: removes the last byte entirely.
    /// When even: overwrites the low nibble of the last byte with the freed sentinel (0x0F).
    /// Decrements <see cref="DigitCount"/> in both cases.
    /// </summary>
    internal void ShrinkDigits()
    {
        lock (_syncRoot)
        {
            if (_digitCount % 2 == 1)
            {
                _bytes.RemoveAt(_bytes.Count - 1);
            }
            else
            {
                GetBitwise(ByteCount - 1, out byte high, out byte _);
                SetBitwise(ByteCount - 1, high, 0x0F);
            }
            _digitCount--;
        }
    }

    /// <summary>
    /// Removes non-significant leading zeros (digits at the most-significant positions
    /// that are zero). Resets to zero state when the entire value is zero.
    /// </summary>
    public void TrimLeadingZeros()
    {
        lock (_syncRoot)
        {
            // Walk from the most-significant digit downward while it is zero.
            while (_digitCount > 1 && GetDigit(_digitCount - 1) == 0)
                ShrinkDigits();

            // If the only remaining digit is also zero, restore zero state.
            if (_digitCount == 1 && GetDigit(0) == 0)
                Reset();
        }
    }

    /// <summary>Clears all bytes from the backing store.</summary>
    internal void ClearDigits()
    {
        lock (_syncRoot)
        {
            _bytes.Clear();
        }
    }

    /// <summary>
    /// Deep-copies the backing bytes from <paramref name="other"/> into this instance.
    /// No-op when <paramref name="other"/> is the same instance or is zero.
    /// Both instance locks are held simultaneously for the duration of the copy,
    /// acquired in canonical order (by <see cref="_id"/>) to prevent ABBA deadlocks.
    /// Uses <see cref="CollectionsMarshal.AsSpan"/> and
    /// <see cref="CollectionsMarshal.SetCount"/> for a zero-allocation copy path.
    /// </summary>
    internal void CopyDigitsFrom(DigitStore other)
    {
        if (ReferenceEquals(other, this))
            return;

        // Acquire both locks in a consistent canonical order to prevent ABBA deadlock
        // when two threads simultaneously call a.CopyDigitsFrom(b) and b.CopyDigitsFrom(a).
        // Instance IDs are assigned at construction via an Interlocked counter, so they
        // are strictly unique — no tie-breaking edge case is possible.
        bool thisFirst    = _id < other._id;
        object firstLock  = thisFirst ? _syncRoot        : other._syncRoot;
        object secondLock = thisFirst ? other._syncRoot  : _syncRoot;

        lock (firstLock)
        lock (secondLock)
        {
            if (other._isZero)
                return;

            // Zero-allocation copy: read directly from other's backing buffer via a
            // Span; both locks are held so neither list can be mutated during the copy.
            ReadOnlySpan<byte> src = CollectionsMarshal.AsSpan(other._bytes);
            _bytes.Clear();
            CollectionsMarshal.SetCount(_bytes, src.Length);
            src.CopyTo(CollectionsMarshal.AsSpan(_bytes));
        }
    }

    /// <summary>Resets digit count and zero flag without touching the byte list.</summary>
    internal void Initialize()
    {
        lock (_syncRoot)
        {
            _digitCount = 0;
            _isZero = true;
        }
    }

    /// <summary>
    /// Clears all digits and reinitializes to zero state.
    /// No-op when already zero.
    /// </summary>
    internal void Reset()
    {
        lock (_syncRoot)
        {
            if (!_isZero)
            {
                ClearDigits();
                Initialize();
            }
        }
    }

    // -------------------------------------------------------------------------
    // Formatting
    // -------------------------------------------------------------------------

    /// <summary>
    /// Returns the decimal representation of the stored digits, most-significant first.
    /// Returns "0" when <see cref="IsZero"/> is true.
    /// </summary>
    public override string ToString() => ToString('\0');

    /// <summary>
    /// Returns the decimal representation with <paramref name="separator"/> inserted
    /// every three digits counting from the least-significant end (e.g. thousands separator).
    /// When <paramref name="separator"/> is <c>'\0'</c> no separator is inserted.
    /// Returns "0" when <see cref="IsZero"/> is true.
    /// </summary>
    public string ToString(char separator)
    {
        // Snapshot immutable view of the store under lock to enable safe string
        // construction (including any future Parallel.For in Phase 1) outside the lock.
        bool isZero;
        long digitCount;
        byte[] bytesSnapshot;
        lock (_syncRoot)
        {
            isZero = _isZero;
            digitCount = _digitCount;
            bytesSnapshot = _bytes.ToArray();
        }

        if (isZero)
            return "0";

        // Pre-allocate output buffer — one slot per digit, MSB at index 0.
        var chars = new char[digitCount];

        long lastByteIdx = bytesSnapshot.LongLength - 1;

        // Resolve the most-significant byte (always sequential; only 1–2 digits).
        byte a = (byte)((bytesSnapshot[lastByteIdx] & 0xF0) >> 4);
        byte b = (byte)(bytesSnapshot[lastByteIdx] & 0x0F);

        // offset is the chars[] index at which interior bytes start writing.
        int offset;
        if (digitCount % 2 == 0)
        {
            // Even count: last byte holds two significant digits.
            // Low nibble (b)  = digit at position digitCount-1 (MSB) → chars[0]
            // High nibble (a) = digit at position digitCount-2       → chars[1]
            chars[0] = (char)('0' + b);
            chars[1] = (char)('0' + a);
            offset = 2;
        }
        else
        {
            // Odd count: only the high nibble (a) is the most-significant digit.
            chars[0] = (char)('0' + a);
            offset = 1;
        }

        // Fill interior bytes in parallel — each byte index c writes two chars at
        // non-overlapping positions: outputIdx = offset + 2*(lastByteIdx-1-c).
        // Within each interior byte the low nibble (odd-position digit) is more
        // significant and therefore written first (lower output index).
        Parallel.For(0L, lastByteIdx, c =>
        {
            byte ba = (byte)((bytesSnapshot[c] & 0xF0) >> 4); // even-position digit
            byte bb = (byte)((bytesSnapshot[c] & 0x0F));       // odd-position digit (more significant)
            int outputIdx = offset + (int)(2 * (lastByteIdx - 1 - c));
            chars[outputIdx]     = (char)('0' + bb);
            chars[outputIdx + 1] = (char)('0' + ba);
        });

        // Insert separator every 3 digits from the right
        string raw = new string(chars);
        if (separator == '\0')
            return raw;
        int len = raw.Length;
        var result = new StringBuilder(len + len / 3);
        for (int i = 0; i < len; i++)
        {
            int distFromRight = len - 1 - i;
            result.Append(raw[i]);
            if (distFromRight > 0 && distFromRight % 3 == 0)
                result.Append(separator);
        }
        return result.ToString();
    }

    /// <summary>Debug helper — prints internal state: size, DigitCount, IsZero, raw nibble pairs.</summary>
    public void Dump(bool showNibbles = false)
    {
        Console.WriteLine($"ByteCount    : {ByteCount}");
        Console.WriteLine($"DigitCount   : {DigitCount}");
        Console.WriteLine($"IsZero       : {IsZero}");
        Console.WriteLine($"Value        : {ToString('.')}");
        if (showNibbles)
        {
            for (int c = 0; c < _bytes.Count; c++)
            {
                Console.WriteLine($"_bytes[{c}] low  (1): {_bytes[c] & 0x0F}");
                Console.WriteLine($"_bytes[{c}] high (2): {(_bytes[c] & 0xF0) >> 4}");
            }
        }
        Console.WriteLine();
    }
}
