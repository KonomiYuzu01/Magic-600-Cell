// c9pp_control.exe: Godot-free baseline of the route-B probe, and its --cpu check.
//
//   c9pp_control.exe --cpu
//       Argument validation, ABI checks with NULL or invalid input, and the frame-code
//       encode/decode round trip. Creates no device and no visible window.
//   c9pp_control.exe --out DIR [--scale S] [--topmost] [--debug-layer] [--recreate-target]
//                    [--flip-sequential] [--global-device]
//       A plain Win32 window (WS_EX_NOREDIRECTIONBITMAP, per-monitor-v2) whose own composition
//       swap chain (premultiplied alpha, a stand-in for Godot's UI) sits in the topmost=TRUE
//       target, with c9pp's core layer beneath it in the topmost=FALSE target, running the
//       same timeline as the Godot harness. Writes DIR/core.jsonl, DIR/meta.json, DIR/result.json.
//
// Exit codes: 0 completed (or --cpu passed), 1 --cpu failed, 2 harness error, 3 suspected TDR.
#include "../c9pp.h"

#include <windows.h>
#include <d3d12.h>
#include <dcomp.h>
#include <dxgi1_6.h>
#include <wrl/client.h>

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cwchar>
#include <string>
#include <vector>

using Microsoft::WRL::ComPtr;

namespace {

// ---------------------------------------------------------------------------------------------
// Layout and colours shared with the Godot harness (project/Probe.cs, Layout).
constexpr uint32_t kBase = 0x18408Cu;   // core layer background
constexpr uint32_t kUi = 0xE67814u;     // opaque UI panel
constexpr uint32_t kFrame = 0x3C3C3Cu;  // opaque host frame around the core rectangle
constexpr uint32_t kCell = 24;
constexpr int kWidth = 1280, kHeight = 720, kResizeWidth = 1024, kResizeHeight = 640;

struct Box {
    int x, y, w, h;
};
struct Layout {
    Box core, panel, ui_sample, bg_sample;
};

Layout layout(int width, int height) {
    const int m = 40;
    Layout l{};
    l.core = {m, m, width - 2 * m, height - 2 * m};
    l.panel = {l.core.x + (l.core.w - 360) / 2, l.core.y + (l.core.h - 200) / 2, 360, 200};
    l.ui_sample = {l.panel.x + 16, l.panel.y + 16, 160, 80};
    l.bg_sample = {l.core.x + 24, l.core.y + l.core.h / 2 - 24, 48, 48};
    return l;
}

// ---------------------------------------------------------------------------------------------
struct Options {
    bool cpu = false;
    std::wstring out;
    double scale = 1.0;
    bool topmost = false, debug_layer = false, recreate_target = false, flip_sequential = false, global_device = false;
};

bool parse(const std::vector<std::wstring>& args, Options& o, std::wstring& error) {
    o = Options{};
    bool seen_out = false, seen_scale = false;
    std::vector<std::wstring> seen;
    for (size_t i = 0; i < args.size(); ++i) {
        const std::wstring& a = args[i];
        if (std::find(seen.begin(), seen.end(), a) != seen.end()) {
            error = L"duplicate option " + a;
            return false;
        }
        seen.push_back(a);
        if (a == L"--cpu") {
            o.cpu = true;
        } else if (a == L"--out" || a == L"--scale") {
            if (i + 1 >= args.size() || args[i + 1].empty() || args[i + 1].rfind(L"--", 0) == 0) {
                error = a + L" needs a value";
                return false;
            }
            const std::wstring& v = args[++i];
            if (a == L"--out") {
                o.out = v;
                seen_out = true;
            } else {
                wchar_t* end = nullptr;
                const double s = std::wcstod(v.c_str(), &end);
                if (!end || *end != 0 || !std::isfinite(s) || s < 0.25 || s > 10.0) {
                    error = L"--scale must be a number in [0.25, 10]";
                    return false;
                }
                o.scale = s;
                seen_scale = true;
            }
        } else if (a == L"--topmost") {
            o.topmost = true;
        } else if (a == L"--debug-layer") {
            o.debug_layer = true;
        } else if (a == L"--recreate-target") {
            o.recreate_target = true;
        } else if (a == L"--flip-sequential") {
            o.flip_sequential = true;
        } else if (a == L"--global-device") {
            o.global_device = true;
        } else {
            error = L"unknown option " + a;
            return false;
        }
    }
    if (o.cpu) {
        if (args.size() != 1) {
            error = L"--cpu takes no other options";
            return false;
        }
        return true;
    }
    if (!seen_out) {
        error = L"--out DIR is required";
        return false;
    }
    if (o.debug_layer && o.global_device) {
        error = L"--debug-layer needs the device factory; it cannot combine with --global-device";
        return false;
    }
    (void)seen_scale;
    return true;
}

std::string utf8(const std::wstring& w) {
    if (w.empty()) return {};
    const int n = WideCharToMultiByte(CP_UTF8, 0, w.c_str(), static_cast<int>(w.size()), nullptr, 0, nullptr, nullptr);
    std::string s(static_cast<size_t>(std::max(n, 0)), '\0');
    if (n > 0) WideCharToMultiByte(CP_UTF8, 0, w.c_str(), static_cast<int>(w.size()), s.data(), n, nullptr, nullptr);
    return s;
}

std::string json_string(const std::string& v) {
    std::string out = "\"";
    for (char ch : v) {
        const auto u = static_cast<unsigned char>(ch);
        if (ch == '"' || ch == '\\') {
            out += '\\';
            out += ch;
        } else if (u < 0x20) {
            char esc[8];
            std::snprintf(esc, sizeof(esc), "\\u%04x", u);
            out += esc;
        } else {
            out += ch;
        }
    }
    return out + "\"";
}

std::string last_error(void* ctx) {
    char buffer[1024] = {};
    c9pp_last_error(ctx, buffer, sizeof(buffer));
    return buffer;
}

// ---------------------------------------------------------------------------------------------
// --cpu check
int g_checks = 0, g_failures = 0;
void expect(bool ok, const char* what) {
    ++g_checks;
    if (!ok) {
        ++g_failures;
        std::printf("FAIL %s\n", what);
    }
}

c9pp_attach_desc valid_desc(HWND hwnd) {
    c9pp_attach_desc d{};
    d.struct_size = sizeof(d);
    d.flags = C9PP_ATTACH_USE_LUID;
    d.hwnd = reinterpret_cast<uint64_t>(hwnd);
    d.x = 40;
    d.y = 40;
    d.width = 1200;
    d.height = 640;
    d.cell_px = kCell;
    d.base_rgb = kBase;
    return d;
}

struct Image {
    uint32_t w = 0, h = 0;
    std::vector<uint8_t> px;
    Image(uint32_t width, uint32_t height, uint32_t rgb) : w(width), h(height), px(static_cast<size_t>(width) * height * 4) {
        for (size_t i = 0; i < px.size(); i += 4) {
            px[i] = static_cast<uint8_t>(rgb);
            px[i + 1] = static_cast<uint8_t>(rgb >> 8);
            px[i + 2] = static_cast<uint8_t>(rgb >> 16);
            px[i + 3] = 255;
        }
    }
    uint8_t* at(uint32_t x, uint32_t y) { return px.data() + (static_cast<size_t>(y) * w + x) * 4; }
    const uint8_t* at(uint32_t x, uint32_t y) const { return px.data() + (static_cast<size_t>(y) * w + x) * 4; }
    void fill(int x0, int y0, int x1, int y1, uint32_t rgb) {
        for (int y = std::max(y0, 0); y < std::min(y1, static_cast<int>(h)); ++y)
            for (int x = std::max(x0, 0); x < std::min(x1, static_cast<int>(w)); ++x) {
                uint8_t* p = at(static_cast<uint32_t>(x), static_cast<uint32_t>(y));
                p[0] = static_cast<uint8_t>(rgb);
                p[1] = static_cast<uint8_t>(rgb >> 8);
                p[2] = static_cast<uint8_t>(rgb >> 16);
            }
    }
};

Image render(uint32_t frame, uint32_t epoch, uint32_t w, uint32_t h) {
    Image img(w, h, 0);
    expect(c9pp_code_render(frame, epoch, kBase, kCell, img.px.data(), w, h, w * 4) == C9PP_OK, "code_render");
    return img;
}

c9pp_decode_pair decode(const Image& img, double ox, double oy, double sx, double sy, uint32_t w, uint32_t h,
                        uint32_t cell = kCell) {
    c9pp_decode_pair p{};
    p.struct_size = sizeof(p);
    const int32_t s = c9pp_code_decode(img.px.data(), img.w, img.h, img.w * 4, ox, oy, sx, sy, w, h, cell, &p);
    expect(s == C9PP_OK, "code_decode status");
    return p;
}

Image scaled(const Image& src, double s, bool bilinear) {
    const uint32_t w = static_cast<uint32_t>(std::lround(src.w * s)), h = static_cast<uint32_t>(std::lround(src.h * s));
    Image out(w, h, 0);
    for (uint32_t y = 0; y < h; ++y)
        for (uint32_t x = 0; x < w; ++x) {
            uint8_t* d = out.at(x, y);
            if (!bilinear) {
                const uint32_t sx = std::min(static_cast<uint32_t>(x / s), src.w - 1), sy = std::min(static_cast<uint32_t>(y / s), src.h - 1);
                std::memcpy(d, src.at(sx, sy), 4);
                continue;
            }
            const double fx = std::clamp((x + 0.5) / s - 0.5, 0.0, src.w - 1.0), fy = std::clamp((y + 0.5) / s - 0.5, 0.0, src.h - 1.0);
            const uint32_t x0 = static_cast<uint32_t>(fx), y0 = static_cast<uint32_t>(fy);
            const uint32_t x1 = std::min(x0 + 1, src.w - 1), y1 = std::min(y0 + 1, src.h - 1);
            const double ax = fx - x0, ay = fy - y0;
            for (int ch = 0; ch < 4; ++ch) {
                const double v = (1 - ax) * (1 - ay) * src.at(x0, y0)[ch] + ax * (1 - ay) * src.at(x1, y0)[ch] +
                                 (1 - ax) * ay * src.at(x0, y1)[ch] + ax * ay * src.at(x1, y1)[ch];
                d[ch] = static_cast<uint8_t>(std::lround(v));
            }
        }
    return out;
}

bool same(const c9pp_decode_pair& p, int status, uint32_t frame, uint32_t epoch) {
    return p.status == status && p.frame_id == frame && p.epoch == epoch;
}

int cpu_check() {
    // 1. Argument validation of this executable.
    struct Case {
        std::vector<std::wstring> args;
        bool ok;
    };
    const Case cases[] = {
        {{L"--cpu"}, true},
        {{}, false},
        {{L"--out"}, false},
        {{L"--out", L"x"}, true},
        {{L"--out", L"x", L"--scale", L"0"}, false},
        {{L"--out", L"x", L"--scale", L"abc"}, false},
        {{L"--out", L"x", L"--scale", L"nan"}, false},
        {{L"--out", L"x", L"--scale", L"11"}, false},
        {{L"--out", L"x", L"--scale", L"1.5"}, true},
        {{L"--out", L"x", L"--bogus"}, false},
        {{L"--cpu", L"--out", L"x"}, false},
        {{L"--out", L"x", L"--out", L"y"}, false},
        {{L"--out", L"--topmost"}, false},
        {{L"--out", L"x", L"--topmost", L"--scale", L"2"}, true},
        {{L"--out", L"x", L"--debug-layer", L"--global-device"}, false},
        {{L"--out", L"x", L"--recreate-target", L"--flip-sequential"}, true},
    };
    for (const Case& c : cases) {
        Options o;
        std::wstring error;
        const bool ok = parse(c.args, o, error);
        expect(ok == c.ok, "argument case");
        if (ok != c.ok) {
            std::string joined;
            for (const auto& a : c.args) joined += utf8(a) + " ";
            std::printf("  arguments: %s(error: %s)\n", joined.c_str(), utf8(error).c_str());
        }
    }
    Options scale_case;
    std::wstring scale_error;
    parse({L"--out", L"x", L"--scale", L"1.5"}, scale_case, scale_error);
    expect(scale_case.scale == 1.5 && scale_case.out == L"x", "parsed values");

    // 2. ABI and NULL handling; none of these creates a device.
    expect(c9pp_abi_version() == C9PP_ABI_VERSION, "abi version");
    char text[256];
    expect(c9pp_last_error(nullptr, nullptr, 0) == C9PP_E_INVALID_ARGUMENT, "last_error without buffer");
    c9pp_frame_record fr{};
    fr.struct_size = sizeof(fr);
    c9pp_stats_row sr{};
    sr.struct_size = sizeof(sr);
    c9pp_sample_desc sd{};
    sd.struct_size = sizeof(sd);
    c9pp_screen_sample ss{};
    ss.struct_size = sizeof(ss);
    uint32_t epoch = 0;
    int32_t hr = 0;
    expect(c9pp_frame(nullptr, 1, 1, &fr) == C9PP_E_INVALID_ARGUMENT, "frame NULL ctx");
    expect(c9pp_poll(nullptr, &sr) == C9PP_E_INVALID_ARGUMENT, "poll NULL ctx");
    expect(c9pp_resize(nullptr, 0, 0, 640, 480) == C9PP_E_INVALID_ARGUMENT, "resize NULL ctx");
    expect(c9pp_remove_device(nullptr) == C9PP_E_INVALID_ARGUMENT, "remove NULL ctx");
    expect(c9pp_recover(nullptr, 0, &epoch) == C9PP_E_INVALID_ARGUMENT, "recover NULL ctx");
    expect(c9pp_sample_screen(nullptr, &sd, &ss) == C9PP_E_INVALID_ARGUMENT, "sample NULL ctx");
    expect(c9pp_device_status(nullptr, &hr) == C9PP_E_INVALID_ARGUMENT, "device_status NULL ctx");
    expect(c9pp_set_phase(nullptr, "steady") == C9PP_E_INVALID_ARGUMENT, "set_phase NULL ctx");
    expect(c9pp_note(nullptr, "x", "{}") == C9PP_E_INVALID_ARGUMENT, "note NULL ctx");
    expect(c9pp_detach(nullptr) == C9PP_E_INVALID_ARGUMENT, "detach NULL ctx");
    expect(c9pp_godot_device_status(nullptr, 0, &hr) == C9PP_E_INVALID_ARGUMENT, "godot status device 0");
    expect(c9pp_last_error(nullptr, text, sizeof(text)) == C9PP_OK && std::strstr(text, "device is 0") != nullptr,
           "last_error text");

    // 3. Attach validation (pure; c9pp_attach must refuse before any device is created).
    const HWND desktop = GetDesktopWindow();
    HWND gone = CreateWindowExW(0, L"STATIC", L"", 0, 0, 0, 1, 1, HWND_MESSAGE, nullptr, nullptr, nullptr);
    expect(gone != nullptr, "message window");
    DestroyWindow(gone);
    const c9pp_attach_desc good = valid_desc(desktop);
    expect(c9pp_validate_attach(&good) == C9PP_OK, "valid desc");
    {
        c9pp_attach_desc d = good;
        d.cell_px = 0;
        expect(c9pp_validate_attach(&d) == C9PP_OK, "cell 0 means 24");
        d = good;
        d.width = 432;
        d.height = 432;
        expect(c9pp_validate_attach(&d) == C9PP_OK, "minimum rectangle");
        d = good;
        d.flags = 0;
        d.adapter = 0x1000;
        expect(c9pp_validate_attach(&d) == C9PP_OK, "adapter pointer form");
    }
    struct Bad {
        const char* name;
        void (*mutate)(c9pp_attach_desc&, HWND);
    };
    const Bad bad[] = {
        {"struct_size", [](c9pp_attach_desc& d, HWND) { d.struct_size -= 1; }},
        {"unknown flag", [](c9pp_attach_desc& d, HWND) { d.flags |= 0x100u; }},
        {"hwnd 0", [](c9pp_attach_desc& d, HWND) { d.hwnd = 0; }},
        {"destroyed hwnd", [](c9pp_attach_desc& d, HWND g) { d.hwnd = reinterpret_cast<uint64_t>(g); }},
        {"luid and adapter", [](c9pp_attach_desc& d, HWND) { d.adapter = 0x1000; }},
        {"neither luid nor adapter", [](c9pp_attach_desc& d, HWND) { d.flags = 0; }},
        {"debug with global", [](c9pp_attach_desc& d, HWND) { d.flags |= C9PP_ATTACH_DEBUG_LAYER | C9PP_ATTACH_GLOBAL_DEVICE; }},
        {"cell 23", [](c9pp_attach_desc& d, HWND) { d.cell_px = 23; }},
        {"cell 65", [](c9pp_attach_desc& d, HWND) { d.cell_px = 65; }},
        {"cell 64 too large for 640", [](c9pp_attach_desc& d, HWND) { d.cell_px = 64; }},
        {"latency 4", [](c9pp_attach_desc& d, HWND) { d.max_latency = 4; }},
        {"timeout 1001", [](c9pp_attach_desc& d, HWND) { d.wait_timeout_ms = 1001; }},
        {"base above 24 bits", [](c9pp_attach_desc& d, HWND) { d.base_rgb = 0x1000000u; }},
        {"grey base", [](c9pp_attach_desc& d, HWND) { d.base_rgb = 0x808080u; }},
        {"negative x", [](c9pp_attach_desc& d, HWND) { d.x = -1; }},
        {"width 431", [](c9pp_attach_desc& d, HWND) { d.width = 431; }},
        {"height 0", [](c9pp_attach_desc& d, HWND) { d.height = 0; }},
        {"width 16385", [](c9pp_attach_desc& d, HWND) { d.width = 16385; }},
    };
    for (const Bad& b : bad) {
        c9pp_attach_desc d = good;
        b.mutate(d, gone);
        const bool refused = c9pp_validate_attach(&d) == C9PP_E_INVALID_ARGUMENT;
        expect(refused, "invalid desc refused");
        if (!refused) std::printf("  case: %s\n", b.name);
        void* ctx = nullptr;
        c9pp_attach_info info{};
        info.struct_size = sizeof(info);
        expect(c9pp_attach(&d, nullptr, &info, &ctx) == C9PP_E_INVALID_ARGUMENT && ctx == nullptr, "attach refuses invalid desc");
    }
    {
        void* ctx = nullptr;
        c9pp_attach_info info{};
        info.struct_size = sizeof(info);
        expect(c9pp_attach(nullptr, nullptr, &info, &ctx) == C9PP_E_INVALID_ARGUMENT && ctx == nullptr, "attach NULL desc");
        info.struct_size = 1;
        expect(c9pp_attach(&good, nullptr, &info, &ctx) == C9PP_E_INVALID_ARGUMENT && ctx == nullptr, "attach bad info size");
        expect(c9pp_attach(&good, nullptr, &info, nullptr) == C9PP_E_INVALID_ARGUMENT, "attach NULL out");
    }

    // 4. Frame code.
    const uint8_t vector[] = {'1', '2', '3', '4', '5', '6', '7', '8', '9'};
    expect(c9pp_crc16(vector, sizeof(vector)) == 0x29B1u, "CRC-16/CCITT-FALSE check value");
    {
        std::vector<uint8_t> tiny(64 * 64 * 4);
        expect(c9pp_code_render(1, 1, kBase, kCell, tiny.data(), 64, 64, 256) == C9PP_E_INVALID_ARGUMENT, "render too small");
        c9pp_decode_pair p{};
        p.struct_size = sizeof(p);
        expect(c9pp_code_decode(tiny.data(), 64, 64, 256, 0, 0, 0.0, 1.0, 640, 480, kCell, &p) == C9PP_E_INVALID_ARGUMENT,
               "decode zero scale");
    }
    const uint32_t W = 640, H = 480;
    const uint32_t codes[][2] = {{0, 0}, {1, 1}, {0xFFFFFFFFu, 255}, {0x12345678u, 2}, {0x80000001u, 128}, {4242, 7}};
    for (const auto& code : codes) {
        const Image img = render(code[0], code[1], W, H);
        expect(same(decode(img, 0, 0, 1, 1, W, H), C9PP_SAMPLE_MATCH, code[0], code[1]), "round trip");
        // Embedded in a larger capture with a UI-coloured surround.
        Image big(800, 640, kUi);
        for (uint32_t y = 0; y < H; ++y) std::memcpy(big.at(37, 53 + y), img.at(0, y), static_cast<size_t>(W) * 4);
        expect(same(decode(big, 37, 53, 1, 1, W, H), C9PP_SAMPLE_MATCH, code[0], code[1]), "embedded round trip");
    }
    {
        const Image img = render(0xC9C9C9C9u, 3, W, H);
        expect(same(decode(scaled(img, 1.25, false), 0, 0, 1.25, 1.25, W, H), C9PP_SAMPLE_MATCH, 0xC9C9C9C9u, 3), "scale 1.25");
        expect(same(decode(scaled(img, 1.5, true), 0, 0, 1.5, 1.5, W, H), C9PP_SAMPLE_MATCH, 0xC9C9C9C9u, 3), "scale 1.5 bilinear");
        expect(same(decode(scaled(img, 2.0, true), 0, 0, 2.0, 2.0, W, H), C9PP_SAMPLE_MATCH, 0xC9C9C9C9u, 3), "scale 2 bilinear");
        // Mild colour shift (colour management) keeps the code.
        Image shifted = img;
        for (size_t i = 0; i < shifted.px.size(); i += 4) {
            shifted.px[i] = static_cast<uint8_t>(std::max(0, shifted.px[i] - 8));
            shifted.px[i + 2] = static_cast<uint8_t>(std::min(255, shifted.px[i + 2] + 10));
        }
        expect(same(decode(shifted, 0, 0, 1, 1, W, H), C9PP_SAMPLE_MATCH, 0xC9C9C9C9u, 3), "colour shift");
    }
    {
        // Corner A: data cell 5 flipped -> bad check; corner B intact -> one corner.
        Image img = render(77, 1, W, H);
        const int c = static_cast<int>(kCell), ax = c, ay = c;
        const uint8_t* cell5 = img.at(static_cast<uint32_t>(ax + 5 * c + c / 2), static_cast<uint32_t>(ay + c / 2));
        const uint32_t flipped = cell5[0] > 128 ? 0x0C0C0Cu : 0xEBEBEBu;
        img.fill(ax + 5 * c, ay, ax + 6 * c, ay + c, flipped);
        c9pp_decode_pair p = decode(img, 0, 0, 1, 1, W, H);
        expect(p.corner_status[0] == C9PP_CORNER_BAD_CHECK && same(p, C9PP_SAMPLE_ONE_CORNER, 77, 1), "flipped data cell");
        // Corner B: sync cell 56 flipped -> bad sync; corner A already bad -> undecodable.
        const int bx = static_cast<int>(W) - c - 8 * c, by = static_cast<int>(H) - c - 8 * c;
        img.fill(bx, by + 7 * c, bx + c, by + 8 * c, 0x0C0C0Cu);
        p = decode(img, 0, 0, 1, 1, W, H);
        expect(p.corner_status[1] == C9PP_CORNER_BAD_SYNC && p.status == C9PP_SAMPLE_UNDECODABLE, "flipped sync cell");
    }
    {
        // A UI panel over corner A -> ambiguous corner; the other corner still decodes.
        Image img = render(31337, 9, W, H);
        img.fill(0, 0, 300, 300, kUi);
        c9pp_decode_pair p = decode(img, 0, 0, 1, 1, W, H);
        expect(p.corner_status[0] == C9PP_CORNER_AMBIGUOUS && same(p, C9PP_SAMPLE_ONE_CORNER, 31337, 9), "covered corner");
        // Base colour only (core layer present, code absent) and host frame grey.
        const Image base(W, H, kBase);
        expect(decode(base, 0, 0, 1, 1, W, H).status == C9PP_SAMPLE_UNDECODABLE, "base only");
        const Image grey(W, H, kFrame);
        expect(decode(grey, 0, 0, 1, 1, W, H).status == C9PP_SAMPLE_UNDECODABLE, "frame grey only");
        // Two corners from different frames.
        Image mixed = render(100, 1, W, H);
        const Image other = render(101, 1, W, H);
        for (uint32_t y = H / 2; y < H; ++y)
            std::memcpy(mixed.at(W / 2, y), other.at(W / 2, y), static_cast<size_t>(W / 2) * 4);
        p = decode(mixed, 0, 0, 1, 1, W, H);
        expect(p.status == C9PP_SAMPLE_MISMATCH && p.corner_frame[0] == 100 && p.corner_frame[1] == 101, "mismatch");
        // Outside the capture.
        p = decode(render(5, 1, W, H), -300, -300, 1, 1, W, H);
        expect(p.corner_status[0] == C9PP_CORNER_OUT_OF_BOUNDS, "out of bounds");
        // Half-cell misregistration must never yield a wrong code.
        const double half = static_cast<double>(kCell) / 2.0, third = static_cast<double>(kCell) / 3.0;
        for (const double shift : {half, third, -half}) {
            Image wide(W + 64, H + 64, kUi);
            const Image src = render(0x5A5A1234u, 4, W, H);
            for (uint32_t y = 0; y < H; ++y) std::memcpy(wide.at(32, 32 + y), src.at(0, y), static_cast<size_t>(W) * 4);
            p = decode(wide, 32 + shift, 32 + shift, 1, 1, W, H);
            const bool wrong = (p.status == C9PP_SAMPLE_MATCH || p.status == C9PP_SAMPLE_ONE_CORNER) &&
                               (p.frame_id != 0x5A5A1234u || p.epoch != 4);
            expect(!wrong, "misregistration never decodes a wrong code");
        }
    }
    // 5. Layout invariants used by both harnesses.
    for (const auto& size : {std::pair{kWidth, kHeight}, std::pair{kResizeWidth, kResizeHeight}}) {
        const Layout l = layout(size.first, size.second);
        const int g = 8 * static_cast<int>(kCell), m = static_cast<int>(kCell);
        const Box codes2[2] = {{l.core.x + m, l.core.y + m, g, g}, {l.core.x + l.core.w - m - g, l.core.y + l.core.h - m - g, g, g}};
        auto overlap = [](const Box& a, const Box& b) {
            return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
        };
        expect(!overlap(l.panel, codes2[0]) && !overlap(l.panel, codes2[1]), "panel clear of codes");
        expect(!overlap(l.bg_sample, codes2[0]) && !overlap(l.bg_sample, codes2[1]) && !overlap(l.bg_sample, l.panel),
               "background sample clear");
        c9pp_attach_desc d = valid_desc(desktop);
        d.x = l.core.x;
        d.y = l.core.y;
        d.width = static_cast<uint32_t>(l.core.w);
        d.height = static_cast<uint32_t>(l.core.h);
        expect(c9pp_validate_attach(&d) == C9PP_OK, "layout rectangle valid");
    }
    std::printf("c9pp_control --cpu: %s (%d checks, %d failed)\n", g_failures ? "FAIL" : "PASS", g_checks, g_failures);
    return g_failures ? 1 : 0;
}

// ---------------------------------------------------------------------------------------------
// GPU baseline.
bool g_closed = false;

LRESULT CALLBACK window_proc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam) {
    switch (message) {
        case WM_CLOSE:
            g_closed = true;
            return 0;
        case WM_ERASEBKGND:
            return 1;
        case WM_PAINT:
            ValidateRect(hwnd, nullptr);
            return 0;
        default:
            return DefWindowProcW(hwnd, message, wparam, lparam);
    }
}

struct Failure {
    int code;
    std::string text;
};
[[noreturn]] void fail(int code, std::string text) { throw Failure{code, std::move(text)}; }
void check(HRESULT hr, const char* what) {
    if (FAILED(hr)) {
        char b[16];
        std::snprintf(b, sizeof(b), "0x%08lX", static_cast<unsigned long>(hr));
        const bool removed = hr == DXGI_ERROR_DEVICE_REMOVED || hr == DXGI_ERROR_DEVICE_RESET || hr == DXGI_ERROR_DEVICE_HUNG;
        fail(removed ? 3 : 2, std::string(what) + " failed: " + b);
    }
}

double seconds(uint64_t ticks, uint64_t frequency) { return static_cast<double>(ticks) / static_cast<double>(frequency); }
uint64_t now() {
    LARGE_INTEGER v;
    QueryPerformanceCounter(&v);
    return static_cast<uint64_t>(v.QuadPart);
}

// The control's own "UI" layer: a premultiplied composition swap chain in the topmost target.
struct UiLayer {
    static constexpr UINT kCount = 2;
    ComPtr<IDXGIFactory6> factory;
    ComPtr<IDXGIAdapter1> adapter;
    ComPtr<ID3D12Device> device;
    ComPtr<ID3D12CommandQueue> queue;
    ComPtr<ID3D12CommandAllocator> allocators[kCount];
    ComPtr<ID3D12GraphicsCommandList> list;
    ComPtr<ID3D12Fence> fence;
    HANDLE event = nullptr;
    uint64_t value = 0, buffer_value[kCount] = {};
    ComPtr<ID3D12DescriptorHeap> heap;
    UINT stride = 0;
    ComPtr<IDXGISwapChain3> swapchain;
    HANDLE waitable = nullptr;
    ComPtr<ID3D12Resource> buffers[kCount];
    ComPtr<IDCompositionDesktopDevice> dcomp;
    ComPtr<IDCompositionTarget> target;
    ComPtr<IDCompositionVisual2> visual;
    UINT w = 0, h = 0;

    ~UiLayer() {
        if (waitable) CloseHandle(waitable);
        if (event) CloseHandle(event);
    }

    void create(HWND hwnd, UINT width, UINT height) {
        w = width;
        h = height;
        check(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)), "CreateDXGIFactory2");
        check(factory->EnumAdapterByGpuPreference(0, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE, IID_PPV_ARGS(&adapter)),
              "EnumAdapterByGpuPreference");
        // The global singleton device, as Godot creates its own.
        check(D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&device)), "D3D12CreateDevice (UI)");
        D3D12_COMMAND_QUEUE_DESC qd{};
        qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
        check(device->CreateCommandQueue(&qd, IID_PPV_ARGS(&queue)), "CreateCommandQueue (UI)");
        for (auto& a : allocators)
            check(device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&a)), "CreateCommandAllocator (UI)");
        check(device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, allocators[0].Get(), nullptr, IID_PPV_ARGS(&list)),
              "CreateCommandList (UI)");
        check(list->Close(), "Close (UI)");
        check(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence)), "CreateFence (UI)");
        event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        if (!event) fail(2, "CreateEvent (UI) failed");
        D3D12_DESCRIPTOR_HEAP_DESC hd{};
        hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV;
        hd.NumDescriptors = kCount;
        check(device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&heap)), "CreateDescriptorHeap (UI)");
        stride = device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
        DXGI_SWAP_CHAIN_DESC1 d{};
        d.Width = w;
        d.Height = h;
        d.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
        d.SampleDesc.Count = 1;
        d.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
        d.BufferCount = kCount;
        d.Scaling = DXGI_SCALING_STRETCH;
        d.SwapEffect = DXGI_SWAP_EFFECT_FLIP_DISCARD;
        d.AlphaMode = DXGI_ALPHA_MODE_PREMULTIPLIED;
        d.Flags = DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT;
        ComPtr<IDXGISwapChain1> sc1;
        check(factory->CreateSwapChainForComposition(queue.Get(), &d, nullptr, &sc1), "CreateSwapChainForComposition (UI)");
        check(sc1.As(&swapchain), "IDXGISwapChain3 (UI)");
        check(swapchain->SetMaximumFrameLatency(1), "SetMaximumFrameLatency (UI)");
        waitable = swapchain->GetFrameLatencyWaitableObject();
        views();
        check(DCompositionCreateDevice3(nullptr, IID_PPV_ARGS(&dcomp)), "DCompositionCreateDevice3 (UI)");
        check(dcomp->CreateTargetForHwnd(hwnd, TRUE, &target), "CreateTargetForHwnd(hwnd, TRUE) (UI)");
        check(dcomp->CreateVisual(&visual), "CreateVisual (UI)");
        check(visual->SetContent(swapchain.Get()), "SetContent (UI)");
        check(target->SetRoot(visual.Get()), "SetRoot (UI)");
        check(dcomp->Commit(), "Commit (UI)");
    }

    void views() {
        D3D12_CPU_DESCRIPTOR_HANDLE handle = heap->GetCPUDescriptorHandleForHeapStart();
        for (UINT i = 0; i < kCount; ++i) {
            check(swapchain->GetBuffer(i, IID_PPV_ARGS(&buffers[i])), "GetBuffer (UI)");
            device->CreateRenderTargetView(buffers[i].Get(), nullptr, handle);
            handle.ptr += stride;
        }
    }

    void idle() {
        check(queue->Signal(fence.Get(), ++value), "Signal (UI)");
        if (fence->GetCompletedValue() < value) {
            check(fence->SetEventOnCompletion(value, event), "SetEventOnCompletion (UI)");
            if (WaitForSingleObject(event, 2000) != WAIT_OBJECT_0) fail(2, "UI GPU wait timed out");
        }
    }

    void resize(UINT width, UINT height) {
        idle();
        for (auto& b : buffers) b.Reset();
        check(swapchain->ResizeBuffers(kCount, width, height, DXGI_FORMAT_B8G8R8A8_UNORM,
                                       DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT),
              "ResizeBuffers (UI)");
        w = width;
        h = height;
        for (auto& v : buffer_value) v = 0;
        views();
        check(dcomp->Commit(), "Commit (UI resize)");
    }

    void wait() const { WaitForSingleObjectEx(waitable, 100, FALSE); }

    void draw(const Layout& l) {
        const UINT index = swapchain->GetCurrentBackBufferIndex();
        if (buffer_value[index] && fence->GetCompletedValue() < buffer_value[index]) {
            check(fence->SetEventOnCompletion(buffer_value[index], event), "SetEventOnCompletion (UI)");
            if (WaitForSingleObject(event, 1000) != WAIT_OBJECT_0) fail(2, "UI GPU wait timed out");
        }
        check(allocators[index]->Reset(), "allocator Reset (UI)");
        check(list->Reset(allocators[index].Get(), nullptr), "list Reset (UI)");
        D3D12_RESOURCE_BARRIER b{};
        b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
        b.Transition.pResource = buffers[index].Get();
        b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
        b.Transition.StateBefore = D3D12_RESOURCE_STATE_PRESENT;
        b.Transition.StateAfter = D3D12_RESOURCE_STATE_RENDER_TARGET;
        list->ResourceBarrier(1, &b);
        D3D12_CPU_DESCRIPTOR_HANDLE rtv = heap->GetCPUDescriptorHandleForHeapStart();
        rtv.ptr += static_cast<SIZE_T>(index) * stride;
        const float clear[4] = {0, 0, 0, 0};
        list->ClearRenderTargetView(rtv, clear, 0, nullptr);
        auto rgb = [](uint32_t v, float out[4]) {
            out[0] = static_cast<float>(v >> 16 & 255) / 255.0f;
            out[1] = static_cast<float>(v >> 8 & 255) / 255.0f;
            out[2] = static_cast<float>(v & 255) / 255.0f;
            out[3] = 1.0f;
        };
        float frame[4], ui[4];
        rgb(kFrame, frame);
        rgb(kUi, ui);
        const LONG W = static_cast<LONG>(w), H = static_cast<LONG>(h);
        const D3D12_RECT around[4] = {
            {0, 0, W, l.core.y},
            {0, l.core.y + l.core.h, W, H},
            {0, l.core.y, l.core.x, l.core.y + l.core.h},
            {l.core.x + l.core.w, l.core.y, W, l.core.y + l.core.h},
        };
        list->ClearRenderTargetView(rtv, frame, 4, around);
        const D3D12_RECT panel = {l.panel.x, l.panel.y, l.panel.x + l.panel.w, l.panel.y + l.panel.h};
        list->ClearRenderTargetView(rtv, ui, 1, &panel);
        std::swap(b.Transition.StateBefore, b.Transition.StateAfter);
        list->ResourceBarrier(1, &b);
        check(list->Close(), "Close (UI)");
        ID3D12CommandList* lists[] = {list.Get()};
        queue->ExecuteCommandLists(1, lists);
        check(queue->Signal(fence.Get(), ++value), "Signal (UI)");
        buffer_value[index] = value;
        check(swapchain->Present(1, 0), "Present (UI)");
    }
};

void set_client_size(HWND hwnd, int width, int height) {
    const DWORD style = static_cast<DWORD>(GetWindowLongPtrW(hwnd, GWL_STYLE));
    const DWORD ex = static_cast<DWORD>(GetWindowLongPtrW(hwnd, GWL_EXSTYLE));
    RECT r{0, 0, width, height};
    AdjustWindowRectExForDpi(&r, style, FALSE, ex, GetDpiForWindow(hwnd));
    SetWindowPos(hwnd, nullptr, 0, 0, r.right - r.left, r.bottom - r.top, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
}

bool pump() {
    MSG msg;
    while (PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE)) {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    return !g_closed;
}

void write_text(const std::wstring& path, const std::string& text) {
    FILE* f = nullptr;
    if (_wfopen_s(&f, path.c_str(), L"wb") != 0 || !f) return;
    std::fwrite(text.data(), 1, text.size(), f);
    std::fclose(f);
}

struct Run {
    Options o;
    HWND hwnd = nullptr;
    UiLayer ui;
    void* ctx = nullptr;
    uint32_t next_frame = 1;
    uint64_t frames = 0, host_frames = 0, samples = 0, frequency = 0, start = 0, last_sample = 0;
    std::string reason = "completed";
    std::vector<std::string> phases;
    int exit_code = 0;

    void core(int status, const char* what) {
        if (status == C9PP_OK) return;
        const std::string text = std::string(what) + ": " + last_error(ctx);
        fail(status == C9PP_E_DEVICE_REMOVED ? 3 : 2, text);
    }

    void phase(const char* name) {
        phases.push_back(name);
        core(c9pp_set_phase(ctx, name), "c9pp_set_phase");
    }

    void host_status() {
        int32_t hr = 0;
        core(c9pp_godot_device_status(ctx, reinterpret_cast<uint64_t>(ui.device.Get()), &hr), "c9pp_godot_device_status");
        if (hr != 0) fail(3, "suspected real TDR: host device removed reason is not S_OK");
    }

    void frame(uint32_t sync) {
        c9pp_frame_record r{};
        r.struct_size = sizeof(r);
        core(c9pp_frame(ctx, next_frame++, sync, &r), "c9pp_frame");
        ++frames;
    }

    void poll() {
        c9pp_stats_row r{};
        r.struct_size = sizeof(r);
        core(c9pp_poll(ctx, &r), "c9pp_poll");
    }

    void sample(const Layout& l, bool force) {
        const uint64_t t = now();
        if (!force && seconds(t - last_sample, frequency) < 0.1) return;
        last_sample = t;
        c9pp_sample_desc d{};
        d.struct_size = sizeof(d);
        d.ui_x = l.ui_sample.x;
        d.ui_y = l.ui_sample.y;
        d.ui_w = static_cast<uint32_t>(l.ui_sample.w);
        d.ui_h = static_cast<uint32_t>(l.ui_sample.h);
        d.bg_x = l.bg_sample.x;
        d.bg_y = l.bg_sample.y;
        d.bg_w = static_cast<uint32_t>(l.bg_sample.w);
        d.bg_h = static_cast<uint32_t>(l.bg_sample.h);
        c9pp_screen_sample s{};
        s.struct_size = sizeof(s);
        core(c9pp_sample_screen(ctx, &d, &s), "c9pp_sample_screen");
        ++samples;
    }

    Layout current() const {
        RECT r{};
        GetClientRect(hwnd, &r);
        return layout(r.right, r.bottom);
    }

    // One host frame: wait for the UI layer, run the core work, sample, present the UI.
    template <class F>
    void tick(F&& core_work) {
        if (!pump()) fail(2, "window closed");
        ui.wait();
        host_status();
        const Layout l = current();
        core_work();
        poll();
        sample(l, false);
        char note[160];
        std::snprintf(note, sizeof(note), "{\"host_frames\":%llu,\"core_frames\":%llu}",
                      static_cast<unsigned long long>(host_frames), static_cast<unsigned long long>(frames));
        core(c9pp_note(ctx, "host_frame", note), "c9pp_note");
        ui.draw(l);
        ++host_frames;
    }

    void timed(const char* name, double duration, uint32_t sync, int per_frame) {
        phase(name);
        const uint64_t t0 = now();
        while (seconds(now() - t0, frequency) < duration * o.scale)
            tick([&] {
                for (int i = 0; i < per_frame; ++i) frame(sync);
            });
    }

    void hold(double duration, double period) {
        phase("hold");
        const uint64_t t0 = now();
        uint64_t last = 0;
        while (seconds(now() - t0, frequency) < duration * o.scale)
            tick([&] {
                const uint64_t t = now();
                if (!last || seconds(t - last, frequency) >= period * o.scale) {
                    frame(1);
                    last = t;
                }
            });
    }

    void resize(const char* name, int width, int height) {
        phase(name);
        set_client_size(hwnd, width, height);
        const uint64_t t0 = now();
        bool done = false;
        uint64_t t_done = 0;
        while (true) {
            const double elapsed = seconds(now() - t0, frequency);
            if (!done) {
                const Layout l = current();
                const bool settled = l.core.w == width - 80 && l.core.h == height - 80;
                if (settled || elapsed >= 1.0 * o.scale) {
                    RECT r{};
                    GetClientRect(hwnd, &r);
                    char note[160];
                    std::snprintf(note, sizeof(note), "{\"requested\":[%d,%d],\"client\":[%ld,%ld],\"settled\":%s}", width,
                                  height, r.right, r.bottom, settled ? "true" : "false");
                    core(c9pp_note(ctx, "host_resize", note), "c9pp_note");
                    ui.resize(static_cast<UINT>(r.right), static_cast<UINT>(r.bottom));
                    core(c9pp_resize(ctx, l.core.x, l.core.y, static_cast<uint32_t>(l.core.w), static_cast<uint32_t>(l.core.h)),
                         "c9pp_resize");
                    done = true;
                    t_done = now();
                }
                tick([] {});
                continue;
            }
            if (seconds(now() - t_done, frequency) >= 2.0 * o.scale) break;
            tick([&] { frame(1); });
        }
    }

    void loss() {
        timed("loss_pre", 0.5, 1, 1);
        phase("loss");
        const int removed = c9pp_remove_device(ctx);
        if (removed == C9PP_E_UNSUPPORTED) {
            const std::string text = "{\"reason\":" + json_string(last_error(ctx)) + "}";
            core(c9pp_note(ctx, "loss_skipped", text.c_str()), "c9pp_note");
            timed("recovered", 5.0, 1, 1);
            return;
        }
        core(removed, "c9pp_remove_device");
        host_status();  // The host's device must be untouched by the core's removal.
        c9pp_frame_record r{};
        r.struct_size = sizeof(r);
        const int attempt = c9pp_frame(ctx, next_frame++, 1, &r);
        char note[96];
        std::snprintf(note, sizeof(note), "{\"status\":%d}", attempt);
        core(c9pp_note(ctx, "frame_after_removal", note), "c9pp_note");
        c9pp_stats_row sr{};
        sr.struct_size = sizeof(sr);
        c9pp_poll(ctx, &sr);
        sample(current(), true);
        uint32_t epoch = 0;
        core(c9pp_recover(ctx, o.recreate_target ? C9PP_RECOVER_RECREATE_TARGET : 0u, &epoch), "c9pp_recover");
        host_status();
        timed("recovered", 5.0, 1, 1);
    }

    void execute() {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        LARGE_INTEGER f;
        QueryPerformanceFrequency(&f);
        frequency = static_cast<uint64_t>(f.QuadPart);
        start = now();
        WNDCLASSEXW wc{};
        wc.cbSize = sizeof(wc);
        wc.lpfnWndProc = window_proc;
        wc.hInstance = GetModuleHandleW(nullptr);
        wc.hCursor = LoadCursorW(nullptr, IDC_ARROW);
        wc.lpszClassName = L"C9ppControl";
        if (!RegisterClassExW(&wc)) fail(2, "RegisterClassEx failed");
        const DWORD ex = WS_EX_NOREDIRECTIONBITMAP | (o.topmost ? WS_EX_TOPMOST : 0u);
        hwnd = CreateWindowExW(ex, wc.lpszClassName, L"C9 present probe (control)", WS_OVERLAPPEDWINDOW, 40, 40, kWidth,
                               kHeight, nullptr, nullptr, wc.hInstance, nullptr);
        if (!hwnd) fail(2, "CreateWindowEx failed");
        set_client_size(hwnd, kWidth, kHeight);
        ShowWindow(hwnd, SW_SHOWNORMAL);
        SetForegroundWindow(hwnd);  // One request; no foreground-lock workaround.
        pump();
        RECT client{};
        GetClientRect(hwnd, &client);
        ui.create(hwnd, static_cast<UINT>(client.right), static_cast<UINT>(client.bottom));
        const Layout l = layout(client.right, client.bottom);
        c9pp_attach_desc d{};
        d.struct_size = sizeof(d);
        d.flags = (o.debug_layer ? C9PP_ATTACH_DEBUG_LAYER : 0u) | (o.flip_sequential ? C9PP_ATTACH_FLIP_SEQUENTIAL : 0u) |
                  (o.global_device ? C9PP_ATTACH_GLOBAL_DEVICE : 0u);
        d.hwnd = reinterpret_cast<uint64_t>(hwnd);
        d.adapter = reinterpret_cast<uint64_t>(ui.adapter.Get());
        d.host_device = reinterpret_cast<uint64_t>(ui.device.Get());
        d.x = l.core.x;
        d.y = l.core.y;
        d.width = static_cast<uint32_t>(l.core.w);
        d.height = static_cast<uint32_t>(l.core.h);
        d.cell_px = kCell;
        d.max_latency = 2;
        d.wait_timeout_ms = 100;
        d.base_rgb = kBase;
        c9pp_attach_info info{};
        info.struct_size = sizeof(info);
        const std::string log = utf8(o.out + L"\\core.jsonl");
        const int attached = c9pp_attach(&d, log.c_str(), &info, &ctx);
        if (attached != C9PP_OK) fail(2, "c9pp_attach: " + last_error(nullptr));
        DXGI_ADAPTER_DESC1 ad{};
        ui.adapter->GetDesc1(&ad);
        char meta[1024];
        std::snprintf(meta, sizeof(meta),
                      "{\n  \"harness\": \"control\",\n  \"abi_version\": %u,\n  \"adapter\": %s,\n  \"scale\": %.3f,\n"
                      "  \"topmost\": %s,\n  \"process_awareness\": \"per-monitor-v2\",\n  \"device_source\": %d,\n"
                      "  \"device_independent\": %d,\n  \"factory_hr\": %d,\n  \"target_hr\": %d,\n  \"ui_rgb\": [%u, %u, %u],\n"
                      "  \"frame_rgb\": [%u, %u, %u],\n  \"base_rgb\": [%u, %u, %u]\n}\n",
                      c9pp_abi_version(), json_string(utf8(ad.Description)).c_str(), o.scale, o.topmost ? "true" : "false",
                      info.device_source, info.device_independent, info.factory_hr, info.target_hr, kUi >> 16 & 255,
                      kUi >> 8 & 255, kUi & 255, kFrame >> 16 & 255, kFrame >> 8 & 255, kFrame & 255, kBase >> 16 & 255,
                      kBase >> 8 & 255, kBase & 255);
        write_text(o.out + L"\\meta.json", meta);
        const std::string colours = "{\"ui_rgb\":[230,120,20],\"frame_rgb\":[60,60,60],\"base_rgb\":[24,64,140]}";
        core(c9pp_note(ctx, "host_colours", colours.c_str()), "c9pp_note");

        timed("warmup", 2.0, 1, 1);
        timed("steady", 10.0, 1, 1);
        hold(3.0, 0.3);
        timed("burst", 5.0, 0, 3);
        resize("resize_a", kResizeWidth, kResizeHeight);
        resize("resize_b", kWidth, kHeight);
        loss();
        phase("finish");
    }
};

int gpu_run(const Options& o) {
    Run run;
    run.o = o;
    CreateDirectoryW(o.out.c_str(), nullptr);
    SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED);
    try {
        run.execute();
    } catch (const Failure& e) {
        run.exit_code = e.code;
        run.reason = e.text;
    } catch (...) {
        run.exit_code = 2;
        run.reason = "unexpected exception";
    }
    if (run.ctx) {
        if (run.exit_code != 0) {
            const std::string note = "{\"exit_code\":" + std::to_string(run.exit_code) + ",\"reason\":" + json_string(run.reason) + "}";
            c9pp_note(run.ctx, "harness_stop", note.c_str());
        }
        c9pp_detach(run.ctx);
    }
    SetThreadExecutionState(ES_CONTINUOUS);
    std::string phases = "[";
    for (size_t i = 0; i < run.phases.size(); ++i) phases += (i ? "," : "") + json_string(run.phases[i]);
    phases += "]";
    const std::string result = "{\n  \"harness\": \"control\",\n  \"exit_code\": " + std::to_string(run.exit_code) +
                               ",\n  \"reason\": " + json_string(run.reason) + ",\n  \"phases\": " + phases +
                               ",\n  \"core_frames\": " + std::to_string(run.frames) + ",\n  \"host_frames\": " +
                               std::to_string(run.host_frames) + ",\n  \"samples\": " + std::to_string(run.samples) + "\n}\n";
    write_text(o.out + L"\\result.json", result);
    if (run.hwnd) DestroyWindow(run.hwnd);
    std::printf("c9pp_control: exit %d (%s)\n", run.exit_code, run.reason.c_str());
    return run.exit_code;
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
    std::vector<std::wstring> args(argv + 1, argv + argc);
    Options o;
    std::wstring error;
    if (!parse(args, o, error)) {
        std::fprintf(stderr, "c9pp_control: %s\nusage: c9pp_control.exe --cpu | --out DIR [--scale S] [--topmost] "
                             "[--debug-layer] [--recreate-target] [--flip-sequential] [--global-device]\n",
                     utf8(error).c_str());
        return 2;
    }
    if (o.cpu) return cpu_check();
    return gpu_run(o);
}
