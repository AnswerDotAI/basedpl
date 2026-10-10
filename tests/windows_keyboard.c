#include <windows.h>
#include <stdio.h>

static HKL layout;
static int failures;

static void check(UINT key, int mods, int count, WCHAR expected) {
    BYTE state[256] = {0};
    WCHAR text[8] = {0};
    state[VK_SHIFT] = (mods & 1) ? 0x80 : 0;
    state[VK_CONTROL] = (mods & 2) ? 0x80 : 0;
    state[VK_MENU] = (mods & 4) ? 0x80 : 0;
    int n = ToUnicodeEx(key, MapVirtualKeyExW(key, MAPVK_VK_TO_VSC, layout), state, text, 8, 0, layout);
    if (n != count || text[0] != expected) {
        printf("VK %02x modifiers %d: got %d/U+%04x, expected %d/U+%04x\n", key, mods, n, text[0], count, expected);
        failures++;
    }
}

int wmain(int argc, wchar_t **argv) {
    if (argc != 2) return 2;
    layout = LoadKeyboardLayoutW(argv[1], KLF_NOTELLSHELL);
    if (!layout) { printf("LoadKeyboardLayout failed: %lu\n", GetLastError()); return 1; }
    check('A', 0, 1, 'a'); check('A', 1, 1, 'A'); check('A', 2, 1, 1);
    check(VK_OEM_7, 1, 1, '"'); check(VK_OEM_COMMA, 1, 1, '<'); check(VK_OEM_PERIOD, 1, 1, '>');
    check('I', 6, 1, 0x2373); check('9', 7, 1, 0x2079); check('0', 7, 1, 0x236c);
    check('O', 6, -1, 0x25cb); check('M', 0, 1, 0x2297);
    check('O', 6, -1, 0x25cb); check('O', 6, 1, 0x25cb);
    check(VK_OEM_5, 6, -1, 0x236d); check('G', 0, 1, 0x2352);
    check('6', 6, -1, '^'); check(VK_OEM_MINUS, 0, 1, 0x207b);
    check('6', 6, -1, '^'); check('1', 0, 1, 0x00b9);
    check('5', 6, -1, '_'); check('1', 0, 1, 0x2081);
    UnloadKeyboardLayout(layout);
    if (!failures) puts("Windows keyboard translation passed.");
    return failures != 0;
}
