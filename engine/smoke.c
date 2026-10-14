/* Prueba de humo del motor (la compila y la corre engine/check.sh).
 *   smoke.exe https   HTTPS por schannel: comprueba que gnutls carga desde el motor.
 *   smoke.exe d3d11   crea un dispositivo D3D11 y dice qué capa lo sirve.
 *   smoke.exe d3d12   crea un dispositivo D3D12 y dice su nivel máximo (12_2 con D3DMetal). */
#define COBJMACROS
#include <windows.h>
#include <winhttp.h>
#include <d3d11.h>
#include <d3d12.h>
#include <stdio.h>
#include <string.h>

static int https(void)
{
    HINTERNET s = WinHttpOpen(L"barrica", WINHTTP_ACCESS_TYPE_NO_PROXY, NULL, NULL, 0);
    HINTERNET c = s ? WinHttpConnect(s, L"www.codeweavers.com", INTERNET_DEFAULT_HTTPS_PORT, 0) : NULL;
    HINTERNET r = c ? WinHttpOpenRequest(c, L"HEAD", L"/", NULL, NULL, NULL, WINHTTP_FLAG_SECURE) : NULL;
    DWORD status = 0, len = sizeof(status);
    if (r && WinHttpSendRequest(r, NULL, 0, NULL, 0, 0, 0) && WinHttpReceiveResponse(r, NULL))
        WinHttpQueryHeaders(r, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, NULL, &status, &len, NULL);
    else
        printf("https: error %lu\n", GetLastError());
    printf("https: HTTP %lu\n", status);
    return status >= 200 && status < 400 ? 0 : 1;
}

static int d3d11(void)
{
    ID3D11Device *dev;
    IDXGIDevice *dxgi;
    IDXGIAdapter *adapter;
    DXGI_ADAPTER_DESC desc;
    D3D_FEATURE_LEVEL level;
    HRESULT hr = D3D11CreateDevice(NULL, D3D_DRIVER_TYPE_HARDWARE, NULL, 0, NULL, 0, D3D11_SDK_VERSION, &dev, &level, NULL);
    if (FAILED(hr))
    {
        printf("d3d11: error %08lx\n", hr);
        return 1;
    }
    ID3D11Device_QueryInterface(dev, &IID_IDXGIDevice, (void **)&dxgi);
    IDXGIDevice_GetAdapter(dxgi, &adapter);
    IDXGIAdapter_GetDesc(adapter, &desc);
    /* DXMT pasa por winemetal.dll; los demás se distinguen por el fabricante que anuncian. */
    printf("d3d11: %s, %ls, fabricante %04x, nivel %x\n", GetModuleHandleA("winemetal.dll") ? "DXMT" : "sin winemetal",
           desc.Description, desc.VendorId, level);
    return 0;
}

static int d3d12(void)
{
    static const D3D_FEATURE_LEVEL wanted[] = {D3D_FEATURE_LEVEL_12_2, D3D_FEATURE_LEVEL_12_1, D3D_FEATURE_LEVEL_12_0,
                                               D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0};
    D3D12_FEATURE_DATA_FEATURE_LEVELS levels = {ARRAYSIZE(wanted), wanted, 0};
    ID3D12Device *dev;
    HRESULT hr = D3D12CreateDevice(NULL, D3D_FEATURE_LEVEL_11_0, &IID_ID3D12Device, (void **)&dev);
    if (FAILED(hr))
    {
        printf("d3d12: error %08lx\n", hr);
        return 1;
    }
    ID3D12Device_CheckFeatureSupport(dev, D3D12_FEATURE_FEATURE_LEVELS, &levels, sizeof(levels));
    printf("d3d12: nivel máximo %x\n", levels.MaxSupportedFeatureLevel);
    return 0;
}

int main(int argc, char **argv)
{
    const char *mode = argc > 1 ? argv[1] : "https";
    return !strcmp(mode, "d3d11") ? d3d11() : !strcmp(mode, "d3d12") ? d3d12() : https();
}
