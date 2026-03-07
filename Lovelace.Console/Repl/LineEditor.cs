namespace Lovelace.Console.Repl;

/// <summary>
/// A simple interactive line editor with cursor movement and command history.
/// Uses <c>Console.ReadKey(intercept: true)</c> so typed characters are
/// echoed manually, giving the editor full control over what appears on screen.
/// </summary>
public sealed class LineEditor
{
    // -----------------------------------------------------------------
    // History
    // -----------------------------------------------------------------

    private readonly List<string> _history = new();

    // -----------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------

    /// <summary>
    /// Reads a line of input from the console, displaying <paramref name="prompt"/>
    /// before the cursor. Supports:
    /// <list type="bullet">
    ///   <item>Printable character insert at cursor</item>
    ///   <item>Backspace — delete character before cursor</item>
    ///   <item>Delete — delete character at cursor</item>
    ///   <item>Left / Right arrow — move cursor one character</item>
    ///   <item>Home / End — jump to start / end of line</item>
    ///   <item>Up / Down arrow — navigate history</item>
    ///   <item>Ctrl+C — signal exit intent by returning <c>null</c></item>
    ///   <item>Enter — submit the current line</item>
    /// </list>
    /// Non-empty submitted lines are appended to the history buffer.
    /// </summary>
    /// <param name="prompt">Text printed before the editing area.</param>
    /// <returns>The submitted input string, or <c>null</c> if Ctrl+C was pressed.</returns>
    public string? ReadLine(string prompt)
    {
        System.Console.Write(prompt);

        var buffer = new List<char>();
        int cursor = 0;          // logical cursor position within buffer
        int historyIndex = _history.Count; // points one past last entry = "current"

        // Scratch buffer so we can restore a partially-edited line when navigating history
        string? pendingLine = null;

        while (true)
        {
            var key = System.Console.ReadKey(intercept: true);

            // ── Ctrl+C ─────────────────────────────────────────────────────
            if (key.Key == ConsoleKey.C && key.Modifiers.HasFlag(ConsoleModifiers.Control))
            {
                System.Console.WriteLine();
                return null;
            }

            // ── Enter ──────────────────────────────────────────────────────
            if (key.Key == ConsoleKey.Enter)
            {
                System.Console.WriteLine();
                string line = new string(buffer.ToArray());
                if (!string.IsNullOrWhiteSpace(line))
                    _history.Add(line);
                return line;
            }

            // ── Backspace ──────────────────────────────────────────────────
            if (key.Key == ConsoleKey.Backspace)
            {
                if (cursor > 0)
                {
                    buffer.RemoveAt(cursor - 1);
                    cursor--;
                    Redraw(prompt, buffer, cursor);
                }
                continue;
            }

            // ── Delete ─────────────────────────────────────────────────────
            if (key.Key == ConsoleKey.Delete)
            {
                if (cursor < buffer.Count)
                {
                    buffer.RemoveAt(cursor);
                    Redraw(prompt, buffer, cursor);
                }
                continue;
            }

            // ── Navigation ─────────────────────────────────────────────────
            if (key.Key == ConsoleKey.LeftArrow)
            {
                if (cursor > 0)
                {
                    cursor--;
                    SetConsoleCursor(prompt, cursor);
                }
                continue;
            }

            if (key.Key == ConsoleKey.RightArrow)
            {
                if (cursor < buffer.Count)
                {
                    cursor++;
                    SetConsoleCursor(prompt, cursor);
                }
                continue;
            }

            if (key.Key == ConsoleKey.Home)
            {
                cursor = 0;
                SetConsoleCursor(prompt, cursor);
                continue;
            }

            if (key.Key == ConsoleKey.End)
            {
                cursor = buffer.Count;
                SetConsoleCursor(prompt, cursor);
                continue;
            }

            // ── History navigation ──────────────────────────────────────────
            if (key.Key == ConsoleKey.UpArrow)
            {
                if (_history.Count == 0) continue;

                if (historyIndex == _history.Count)
                    pendingLine = new string(buffer.ToArray()); // save current draft

                if (historyIndex > 0)
                {
                    historyIndex--;
                    ReplaceBuffer(buffer, _history[historyIndex], ref cursor);
                    Redraw(prompt, buffer, cursor);
                }
                continue;
            }

            if (key.Key == ConsoleKey.DownArrow)
            {
                if (historyIndex < _history.Count)
                {
                    historyIndex++;
                    string replacement = historyIndex == _history.Count
                        ? (pendingLine ?? string.Empty)
                        : _history[historyIndex];
                    ReplaceBuffer(buffer, replacement, ref cursor);
                    Redraw(prompt, buffer, cursor);
                }
                continue;
            }

            // ── Printable characters ────────────────────────────────────────
            if (!char.IsControl(key.KeyChar))
            {
                // Typing resets history navigation to "live" position
                historyIndex = _history.Count;
                pendingLine = null;

                buffer.Insert(cursor, key.KeyChar);
                cursor++;
                Redraw(prompt, buffer, cursor);
            }
        }
    }

    // -----------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------

    /// <summary>
    /// Redraws the entire prompt + buffer and repositions the cursor.
    /// </summary>
    private static void Redraw(string prompt, List<char> buffer, int cursor)
    {
        // Move to the beginning of the current line, clear it, rewrite.
        int left = System.Console.CursorLeft;
        int top = System.Console.CursorTop;

        // Erase to start of line by going to column 0 and clearing
        System.Console.CursorLeft = 0;
        System.Console.Write(new string(' ', prompt.Length + buffer.Count + 1));
        System.Console.CursorLeft = 0;
        System.Console.Write(prompt);
        System.Console.Write(new string(buffer.ToArray()));

        // Reposition the logical cursor
        System.Console.CursorLeft = prompt.Length + cursor;
    }

    /// <summary>
    /// Moves the console cursor to a logical position within the editing area.
    /// </summary>
    private static void SetConsoleCursor(string prompt, int cursor)
    {
        System.Console.CursorLeft = prompt.Length + cursor;
    }

    /// <summary>
    /// Replaces the content of <paramref name="buffer"/> with <paramref name="text"/>
    /// and moves the logical cursor to the end.
    /// </summary>
    private static void ReplaceBuffer(List<char> buffer, string text, ref int cursor)
    {
        buffer.Clear();
        buffer.AddRange(text);
        cursor = buffer.Count;
    }
}
