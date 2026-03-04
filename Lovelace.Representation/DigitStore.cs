using System.Text;
using System.Runtime.CompilerServices;

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
        _digitCount = other._digitCount;
        _isZero = other._isZero;
        if (!other._isZero)
            CopyDigitsFrom(other);
    }

    // -------------------------------------------------------------------------
    // Public properties
    // -------------------------------------------------------------------------

    /// <summary>Number of bytes in the backing store.</summary>
    public long ByteCount => _bytes.Count;

    /// <summary>Number of logical decimal digits stored.</summary>
    public long DigitCount
    {
        get => _digitCount;
        internal set => _digitCount = value;
    }

    /// <summary>True when the number represented is zero.</summary>
    public bool IsZero
    {
        get => _isZero;
        internal set => _isZero = value;
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
        if (_isZero)
            return 0;

        if (position >= 0 && position < _digitCount)
        {
            GetBitwise(position / 2, out byte high, out byte low);
            return (position % 2 == 0) ? high : low;
        }

        return 0;
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

    /// <summary>Appends a sentinel byte (0x0C) to grow the backing store by one byte.</summary>
    internal void GrowDigits()
    {
        _bytes.Add(0x0C);
    }

    /// <summary>
    /// Removes the most-significant digit slot.
    /// When <see cref="DigitCount"/> is odd: removes the last byte entirely.
    /// When even: overwrites the low nibble of the last byte with the freed sentinel (0x0F).
    /// Decrements <see cref="DigitCount"/> in both cases.
    /// </summary>
    internal void ShrinkDigits()
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

    /// <summary>Clears all bytes from the backing store.</summary>
    internal void ClearDigits()
    {
        _bytes.Clear();
    }

    /// <summary>
    /// Deep-copies the backing bytes from <paramref name="other"/> into this instance.
    /// No-op when <paramref name="other"/> is the same instance or is zero.
    /// </summary>
    internal void CopyDigitsFrom(DigitStore other)
    {
        if (!ReferenceEquals(other, this) && !other._isZero)
        {
            _bytes.Clear();
            _bytes.AddRange(other._bytes);
        }
    }

    /// <summary>Resets digit count and zero flag without touching the byte list.</summary>
    internal void Initialize()
    {
        _digitCount = 0;
        _isZero = true;
    }

    /// <summary>
    /// Clears all digits and reinitializes to zero state.
    /// No-op when already zero.
    /// </summary>
    internal void Reset()
    {
        if (!_isZero)
        {
            ClearDigits();
            Initialize();
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
        if (_isZero)
            return "0";

        // Build raw digit string — most-significant digit first.
        var sb = new StringBuilder((int)_digitCount + 4);

        long lastByteIdx = ByteCount - 1;
        GetBitwise(lastByteIdx, out byte a, out byte b);

        // Handle the most-significant byte
        if (_digitCount % 2 == 0)
        {
            // Even count: last byte holds two significant digits.
            // High nibble (a) is digit at position DigitCount-2 (less significant),
            // Low nibble (b) is digit at position DigitCount-1 (most significant).
            sb.Append((char)('0' + b));
            sb.Append((char)('0' + a));
        }
        else
        {
            // Odd count: only the high nibble (a) is the most-significant digit.
            sb.Append((char)('0' + a));
        }

        // Iterate remaining bytes from second-to-last down to byte 0
        for (long c = lastByteIdx - 1; c >= 0; c--)
        {
            GetBitwise(c, out byte ba, out byte bb);
            // Within each interior byte: low nibble is more significant (higher position)
            sb.Append((char)('0' + bb));
            sb.Append((char)('0' + ba));
        }

        // Insert separator every 3 digits from the right
        if (separator == '\0')
            return sb.ToString();

        string raw = sb.ToString();
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
