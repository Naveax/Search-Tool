// Disposable Shell COM server. No DllRegisterServer or persistent install.
// The fixture must never be registered on a physical Windows machine.
#define SEARCHTOOL_PRIVATE_SHELL_COM_SERVER
#include "private_shell_namespace_test.cpp"
namespace {
class PrivateShellFactory final : public IClassFactory {
public:
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid,void** out) override {
        if (!out) return E_POINTER;
        *out=nullptr;
        if (iid!=IID_IUnknown && iid!=IID_IClassFactory) return E_NOINTERFACE;
        *out=static_cast<IClassFactory*>(this);
        AddRef();return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override {return ++refs_;}
    ULONG STDMETHODCALLTYPE Release() override {
        const ULONG count=--refs_;
        if (!count) delete this;
        return count;
    }
    HRESULT STDMETHODCALLTYPE CreateInstance(IUnknown* outer,REFIID iid,void** out) override {
        if (!out) return E_POINTER;
        *out=nullptr;
        if (outer) return CLASS_E_NOAGGREGATION;
        auto* folder=new (std::nothrow) PrivateFolder();
        if (!folder) return E_OUTOFMEMORY;
        const HRESULT result=folder->QueryInterface(iid,out);
        folder->Release();
        return result;
    }
    HRESULT STDMETHODCALLTYPE LockServer(BOOL) override {
        return S_OK;
    }
private:
    std::atomic<ULONG> refs_{1};
};
}
STDAPI DllGetClassObject(REFCLSID clsid,REFIID iid,void** out) {
    if (!out) return E_POINTER;
    *out=nullptr;
    if (clsid!=kFixtureShellClsid) return CLASS_E_CLASSNOTAVAILABLE;
    auto* factory=new (std::nothrow) PrivateShellFactory();
    if (!factory) return E_OUTOFMEMORY;
    const HRESULT hr=factory->QueryInterface(iid,out);
    factory->Release();
    return hr;
}
STDAPI DllCanUnloadNow() {
    // Conservative: never unload while an untracked Shell folder exists.
    // CI guest process lifetime is short and fully disposable.
    return S_FALSE;
}