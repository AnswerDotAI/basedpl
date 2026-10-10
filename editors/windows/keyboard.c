#include <windows.h>
#define KBD_TYPE 4
#include <kbd.h>
#pragma data_seg(".data")
#include "layout.h"

static USHORT scans[] = {
    T00, T01, T02, T03, T04, T05, T06, T07, T08, T09, T0A, T0B, T0C, T0D, T0E, T0F,
    T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T1A, T1B, T1C, T1D, T1E, T1F,
    T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T2A, T2B, T2C, T2D, T2E, T2F,
    T30, T31, T32, T33, T34, T35, T36 | KBDEXT, T37 | KBDMULTIVK,
    T38, T39, T3A, T3B, T3C, T3D, T3E, T3F, T40, T41, T42, T43, T44,
    T45 | KBDEXT | KBDMULTIVK, T46 | KBDMULTIVK,
    T47 | KBDNUMPAD | KBDSPECIAL, T48 | KBDNUMPAD | KBDSPECIAL, T49 | KBDNUMPAD | KBDSPECIAL, T4A,
    T4B | KBDNUMPAD | KBDSPECIAL, T4C | KBDNUMPAD | KBDSPECIAL, T4D | KBDNUMPAD | KBDSPECIAL, T4E,
    T4F | KBDNUMPAD | KBDSPECIAL, T50 | KBDNUMPAD | KBDSPECIAL, T51 | KBDNUMPAD | KBDSPECIAL,
    T52 | KBDNUMPAD | KBDSPECIAL, T53 | KBDNUMPAD | KBDSPECIAL, T54, T55, T56, T57,
    T58, T59, T5A, T5B, T5C, T5D, T5E, T5F, T60, T61, T62, T63,
    T64, T65, T66, T67, T68, T69, T6A, T6B, T6C, T6D, T6E, T6F,
    T70, T71, T72, T73, T74, T75, T76, T77, T78, T79, T7A, T7B, T7C, T7D, T7E
};

static VSC_VK extended[] = {
    {0x10, X10 | KBDEXT}, {0x19, X19 | KBDEXT}, {0x1d, X1D | KBDEXT}, {0x20, X20 | KBDEXT},
    {0x21, X21 | KBDEXT}, {0x22, X22 | KBDEXT}, {0x24, X24 | KBDEXT}, {0x2e, X2E | KBDEXT},
    {0x30, X30 | KBDEXT}, {0x32, X32 | KBDEXT}, {0x35, X35 | KBDEXT}, {0x37, X37 | KBDEXT},
    {0x38, X38 | KBDEXT}, {0x47, X47 | KBDEXT}, {0x48, X48 | KBDEXT}, {0x49, X49 | KBDEXT},
    {0x4b, X4B | KBDEXT}, {0x4d, X4D | KBDEXT}, {0x4f, X4F | KBDEXT}, {0x50, X50 | KBDEXT},
    {0x51, X51 | KBDEXT}, {0x52, X52 | KBDEXT}, {0x53, X53 | KBDEXT}, {0x5b, X5B | KBDEXT},
    {0x5c, X5C | KBDEXT}, {0x5d, X5D | KBDEXT}, {0x5f, X5F | KBDEXT}, {0x65, X65 | KBDEXT},
    {0x66, X66 | KBDEXT}, {0x67, X67 | KBDEXT}, {0x68, X68 | KBDEXT}, {0x69, X69 | KBDEXT},
    {0x6a, X6A | KBDEXT}, {0x6b, X6B | KBDEXT}, {0x6c, X6C | KBDEXT}, {0x6d, X6D | KBDEXT},
    {0x1c, X1C | KBDEXT}, {0x46, X46 | KBDEXT}, {0}
};
static VSC_VK e1[] = {{0x1d, Y1D}, {0}};
static VK_TO_BIT bits[] = {{VK_SHIFT, KBDSHIFT}, {VK_CONTROL, KBDCTRL}, {VK_MENU, KBDALT}, {0}};
static MODIFIERS modifiers = {bits, 7, {0, 1, 2, 3, SHFT_INVALID, SHFT_INVALID, 4, 5}};
static VK_TO_WCHAR_TABLE tables[] = {{(PVK_TO_WCHARS1)characters, 6, sizeof(characters[0])}, {0}};
static KBDTABLES layout = {
    &modifiers, tables, deadkeys, NULL, NULL, NULL, scans, sizeof(scans) / sizeof(scans[0]),
    extended, e1, MAKELONG(KLLF_ALTGR, KBD_VERSION), 0, 0, NULL
};

PKBDTABLES KbdLayerDescriptor(void) { return &layout; }
