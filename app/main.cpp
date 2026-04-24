#include <windows.h>
#include <tchar.h>

#include <d3d11.h>
#include <dxgi.h>

#include "imgui.h"
#include "imgui_impl_win32.h"
#include "imgui_impl_dx11.h"

#include "whatbubbles_cxx/ffi.h"
#include "app_shell.h"

#include <memory>
#include <string>

extern IMGUI_IMPL_API LRESULT ImGui_ImplWin32_WndProcHandler(HWND hWnd, UINT msg, WPARAM wParam, LPARAM lParam);

namespace {

ID3D11Device*            g_device         = nullptr;
ID3D11DeviceContext*     g_device_context = nullptr;
IDXGISwapChain*          g_swap_chain     = nullptr;
ID3D11RenderTargetView*  g_rtv            = nullptr;

bool create_render_target()
{
    ID3D11Texture2D* back_buffer = nullptr;
    if (FAILED(g_swap_chain->GetBuffer(0, IID_PPV_ARGS(&back_buffer))) || back_buffer == nullptr)
        return false;
    HRESULT hr = g_device->CreateRenderTargetView(back_buffer, nullptr, &g_rtv);
    back_buffer->Release();
    return SUCCEEDED(hr);
}

void cleanup_render_target()
{
    if (g_rtv) { g_rtv->Release(); g_rtv = nullptr; }
}

bool create_device(HWND hwnd)
{
    DXGI_SWAP_CHAIN_DESC desc{};
    desc.BufferCount        = 2;
    desc.BufferDesc.Format   = DXGI_FORMAT_R8G8B8A8_UNORM;
    desc.BufferDesc.RefreshRate.Numerator   = 60;
    desc.BufferDesc.RefreshRate.Denominator = 1;
    desc.BufferUsage        = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    desc.OutputWindow       = hwnd;
    desc.SampleDesc.Count   = 1;
    desc.SampleDesc.Quality = 0;
    desc.Windowed           = TRUE;
    desc.SwapEffect         = DXGI_SWAP_EFFECT_DISCARD;

    UINT flags = 0;
#ifdef _DEBUG
    flags |= D3D11_CREATE_DEVICE_DEBUG;
#endif
    const D3D_FEATURE_LEVEL levels[] = { D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_0 };
    D3D_FEATURE_LEVEL chosen;
    HRESULT hr = D3D11CreateDeviceAndSwapChain(nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr,
        flags, levels, _countof(levels), D3D11_SDK_VERSION,
        &desc, &g_swap_chain, &g_device, &chosen, &g_device_context);
    if (FAILED(hr)) return false;
    return create_render_target();
}

void cleanup_device()
{
    cleanup_render_target();
    if (g_swap_chain)     { g_swap_chain->Release();     g_swap_chain     = nullptr; }
    if (g_device_context) { g_device_context->Release(); g_device_context = nullptr; }
    if (g_device)         { g_device->Release();         g_device         = nullptr; }
}

LRESULT WINAPI wnd_proc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp)
{
    if (ImGui_ImplWin32_WndProcHandler(hwnd, msg, wp, lp))
        return true;
    switch (msg)
    {
    case WM_SIZE:
        if (g_device && wp != SIZE_MINIMIZED)
        {
            cleanup_render_target();
            g_swap_chain->ResizeBuffers(0, (UINT)LOWORD(lp), (UINT)HIWORD(lp), DXGI_FORMAT_UNKNOWN, 0);
            create_render_target();
        }
        return 0;
    case WM_SYSCOMMAND:
        if ((wp & 0xfff0) == SC_KEYMENU) return 0;
        break;
    case WM_DESTROY:
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcW(hwnd, msg, wp, lp);
}

}  // namespace

int APIENTRY wWinMain(HINSTANCE hinstance, HINSTANCE, PWSTR, int)
{
    WNDCLASSEXW wc{ sizeof(wc), CS_CLASSDC, wnd_proc, 0L, 0L,
        hinstance, nullptr, nullptr, nullptr, nullptr, L"WhatBubbles", nullptr };
    RegisterClassExW(&wc);

    HWND hwnd = CreateWindowExW(0, wc.lpszClassName, L"WhatBubbles",
        WS_OVERLAPPEDWINDOW, 100, 100, 1400, 900,
        nullptr, nullptr, hinstance, nullptr);

    if (!create_device(hwnd))
    {
        cleanup_device();
        UnregisterClassW(wc.lpszClassName, hinstance);
        return 1;
    }

    ShowWindow(hwnd, SW_SHOWDEFAULT);
    UpdateWindow(hwnd);

    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGuiIO& io = ImGui::GetIO();
    io.ConfigFlags |= ImGuiConfigFlags_NavEnableKeyboard;

    ImGui::StyleColorsDark();

    const std::string fonts_root = []() -> std::string {
        char buf[MAX_PATH] = {};
        if (GetWindowsDirectoryA(buf, MAX_PATH)) return std::string(buf) + "\\Fonts\\";
        return "C:\\Windows\\Fonts\\";
    }();
    auto try_load_font = [&](const char* name, float size, const ImFontConfig* cfg,
                             const ImWchar* ranges) -> ImFont* {
        const std::string full = fonts_root + name;
        return io.Fonts->AddFontFromFileTTF(full.c_str(), size, cfg, ranges);
    };

    ImFontConfig primary_cfg;
    primary_cfg.OversampleH = 2;
    primary_cfg.OversampleV = 2;
    primary_cfg.PixelSnapH  = false;
    static const ImWchar primary_ranges[] = {
        0x0020, 0x00FF, // Basic Latin + Latin-1
        0x0100, 0x017F, // Latin Extended-A
        0x0180, 0x024F, // Latin Extended-B
        0x0370, 0x03FF, // Greek
        0x0400, 0x04FF, // Cyrillic
        0x2000, 0x206F, // General Punctuation
        0x2070, 0x209F, // Super/subscripts
        0x20A0, 0x20CF, // Currency
        0x2100, 0x214F, // Letterlike
        0x2190, 0x21FF, // Arrows
        0x2200, 0x22FF, // Math operators
        0x25A0, 0x25FF, // Geometric shapes
        0x2600, 0x26FF, // Misc symbols
        0x2700, 0x27BF, // Dingbats
        0,
    };
    ImFont* primary = try_load_font("segoeui.ttf", 17.0f, &primary_cfg, primary_ranges);
    if (!primary) primary = try_load_font("tahoma.ttf", 17.0f, &primary_cfg, primary_ranges);
    if (!primary) io.Fonts->AddFontDefault();

    ImFontConfig symbol_cfg;
    symbol_cfg.MergeMode   = true;
    symbol_cfg.OversampleH = 1;
    symbol_cfg.OversampleV = 1;
    symbol_cfg.GlyphMinAdvanceX = 14.0f;
    static const ImWchar symbol_ranges[] = {
        0x2000, 0x2BFF,   // punct → misc symbols / arrows (BMP)
        0xFB00, 0xFB4F,   // ligatures
        0xFE00, 0xFE0F,   // variation selectors
        0xFF00, 0xFFEF,   // halfwidth/fullwidth
        0,
    };
    try_load_font("seguisym.ttf", 16.0f, &symbol_cfg, symbol_ranges);

    ImGui_ImplWin32_Init(hwnd);
    ImGui_ImplDX11_Init(g_device, g_device_context);

    const std::string data_dir = std::string(whatbubbles::default_data_dir());
    try {
        whatbubbles::init_logger_at(rust::Str(data_dir));
    } catch (const std::exception&) {
        // non-fatal
    }

    std::unique_ptr<app::AppShell> shell;
    std::string init_error;
    try {
        shell = app::AppShell::create(data_dir);
    } catch (const std::exception& e) {
        init_error = std::string("AppShell init failed: ") + e.what();
    }

    const float clear_color[4] = { 0.07f, 0.08f, 0.10f, 1.00f };

    bool done = false;
    while (!done)
    {
        MSG msg;
        while (PeekMessage(&msg, nullptr, 0u, 0u, PM_REMOVE))
        {
            TranslateMessage(&msg);
            DispatchMessage(&msg);
            if (msg.message == WM_QUIT) done = true;
        }
        if (done) break;

        ImGui_ImplDX11_NewFrame();
        ImGui_ImplWin32_NewFrame();
        ImGui::NewFrame();

        if (shell) {
            shell->draw();
        } else {
            ImGui::Begin("WhatBubbles — startup error");
            ImGui::TextWrapped("%s", init_error.c_str());
            ImGui::End();
        }

        ImGui::Render();
        g_device_context->OMSetRenderTargets(1, &g_rtv, nullptr);
        g_device_context->ClearRenderTargetView(g_rtv, clear_color);
        ImGui_ImplDX11_RenderDrawData(ImGui::GetDrawData());
        g_swap_chain->Present(1, 0);
    }

    shell.reset();

    ImGui_ImplDX11_Shutdown();
    ImGui_ImplWin32_Shutdown();
    ImGui::DestroyContext();

    cleanup_device();
    DestroyWindow(hwnd);
    UnregisterClassW(wc.lpszClassName, hinstance);
    return 0;
}
