#include "stages.h"
#include "scene_record.h"
#include "stage_sources.h"
#include <windows.h>
#include <unknwn.h>
#include <objbase.h>
#include <dxcapi.h>
#include <iomanip>
#include <mutex>
#include <sstream>

namespace sa2::stages {
namespace {
std::string hr_text(const char* call, HRESULT result) {
    std::ostringstream text;
    text << call << " failed: HRESULT 0x" << std::hex << std::setw(8) << std::setfill('0') << uint32_t(result);
    return text.str();
}
void check(ID3D12Device* device, HRESULT result, const char* call) {
    if (SUCCEEDED(result)) return;
    throw Failure(FAILED(device->GetDeviceRemovedReason()) ? SA2_E_DEVICE_REMOVED : SA2_E_D3D12, hr_text(call, result));
}
std::string sha256(std::string_view text) {
    return sb::sha256(std::span(reinterpret_cast<const uint8_t*>(text.data()), text.size()));
}
std::string_view embedded_text(const unsigned char* data, size_t size) {
    return {reinterpret_cast<const char*>(data), size};
}

// dxil.dll and dxcompiler.dll from the folder of the module that holds this code (sa2_interop.dll), by absolute path,
// loaded once per process. A failed load is retried on the next compile; a loaded pair is never unloaded.
struct Dxc {
    DxcCreateInstanceProc create = nullptr;
    Json info;
};
std::mutex dxc_lock;
std::optional<Dxc> dxc_loaded;

std::string narrow(const std::wstring& text) {
    std::string out; // the library names are ASCII
    for (const wchar_t c : text) out.push_back(static_cast<char>(c));
    return out;
}
const Dxc& dxc() {
    std::lock_guard lock(dxc_lock);
    if (dxc_loaded) return *dxc_loaded;
    const auto folder = module_path().parent_path();
    HMODULE dxil = nullptr;
    auto load = [&](const wchar_t* name) {
        const auto path = folder / name;
        HMODULE module = LoadLibraryExW(path.c_str(), nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32);
        if (!module) {
            const auto error = GetLastError();
            throw Failure(SA2_E_UNAVAILABLE, "LoadLibraryExW(" + narrow(name) + " beside the core) failed: error "
                + std::to_string(error));
        }
        return module;
    };
    auto entry = [&](HMODULE module, const char* library) {
        auto proc = reinterpret_cast<DxcCreateInstanceProc>(reinterpret_cast<void*>(GetProcAddress(module, "DxcCreateInstance")));
        if (!proc) throw Failure(SA2_E_UNAVAILABLE, std::string(library) + " has no DxcCreateInstance export");
        return proc;
    };
    // dxil.dll first, so that dxcompiler.dll's own load of dxil.dll by name finds this module.
    dxil = load(L"dxil.dll");
    HMODULE compiler_module = nullptr;
    try {
        compiler_module = load(L"dxcompiler.dll");
        Dxc result;
        result.create = entry(compiler_module, "dxcompiler.dll");
        const auto validator_create = entry(dxil, "dxil.dll");
        auto version = [](IUnknown* object, const char* library, Json& out) {
            ComPtr<IDxcVersionInfo> info;
            if (FAILED(object->QueryInterface(IID_PPV_ARGS(&info))))
                throw Failure(SA2_E_UNAVAILABLE, std::string(library) + " has no IDxcVersionInfo");
            UINT32 major = 0, minor = 0;
            if (FAILED(info->GetVersion(&major, &minor))) throw Failure(SA2_E_UNAVAILABLE, std::string(library) + " GetVersion failed");
            out["library"] = library;
            out["version"] = std::to_string(major) + "." + std::to_string(minor);
            ComPtr<IDxcVersionInfo2> commit;
            if (SUCCEEDED(info.As(&commit))) {
                UINT32 count = 0;
                char* hash = nullptr;
                if (SUCCEEDED(commit->GetCommitInfo(&count, &hash))) {
                    out["commit_count"] = count;
                    out["commit_hash"] = hash ? std::string(hash) : std::string();
                    CoTaskMemFree(hash);
                }
            }
            ComPtr<IDxcVersionInfo3> custom;
            if (SUCCEEDED(info.As(&custom))) {
                char* text = nullptr;
                if (SUCCEEDED(custom->GetCustomVersionString(&text)) && text) {
                    out["custom_version"] = std::string(text);
                    CoTaskMemFree(text);
                }
            }
        };
        ComPtr<IDxcCompiler3> compiler;
        if (FAILED(result.create(CLSID_DxcCompiler, IID_PPV_ARGS(&compiler))))
            throw Failure(SA2_E_UNAVAILABLE, "DxcCreateInstance(CLSID_DxcCompiler) failed");
        ComPtr<IDxcValidator> validator;
        if (FAILED(validator_create(CLSID_DxcValidator, IID_PPV_ARGS(&validator))))
            throw Failure(SA2_E_UNAVAILABLE, "DxcCreateInstance(CLSID_DxcValidator) failed");
        Json compiler_info{Json::Object{}}, validator_info{Json::Object{}};
        version(compiler.Get(), "dxcompiler.dll", compiler_info);
        version(validator.Get(), "dxil.dll", validator_info);
        compiler_info["validator"] = validator_info;
        result.info = compiler_info;
        dxc_loaded = std::move(result);
        return *dxc_loaded;
    } catch (...) {
        if (compiler_module) FreeLibrary(compiler_module);
        if (dxil) FreeLibrary(dxil);
        throw;
    }
}

struct Output {
    bool ok = false;
    std::vector<uint8_t> dxil;
    std::string errors;
};
Output run(const Dxc& library, const std::string& source, const Pass& pass) {
    Output out;
    ComPtr<IDxcCompiler3> compiler;
    if (FAILED(library.create(CLSID_DxcCompiler, IID_PPV_ARGS(&compiler))))
        throw Failure(SA2_E_UNAVAILABLE, "DxcCreateInstance(CLSID_DxcCompiler) failed");
    std::vector<std::wstring> wide;
    for (const auto& argument : arguments(pass)) wide.emplace_back(argument.begin(), argument.end()); // ASCII
    std::vector<LPCWSTR> argv;
    for (const auto& argument : wide) argv.push_back(argument.c_str());
    const DxcBuffer buffer{source.data(), source.size(), DXC_CP_UTF8};
    ComPtr<IDxcResult> result;
    HRESULT h = compiler->Compile(&buffer, argv.data(), UINT32(argv.size()), nullptr, IID_PPV_ARGS(&result));
    if (FAILED(h)) {
        out.errors = hr_text("IDxcCompiler3::Compile", h);
        return out;
    }
    HRESULT status = E_FAIL;
    if (FAILED(result->GetStatus(&status))) status = E_FAIL;
    ComPtr<IDxcBlobUtf8> errors;
    if (SUCCEEDED(result->GetOutput(DXC_OUT_ERRORS, IID_PPV_ARGS(&errors), nullptr)) && errors && errors->GetStringLength())
        out.errors.assign(errors->GetStringPointer(), errors->GetStringLength());
    if (FAILED(status)) {
        if (out.errors.empty()) out.errors = hr_text("the compile", status);
        return out;
    }
    ComPtr<IDxcBlob> object;
    if (FAILED(result->GetOutput(DXC_OUT_OBJECT, IID_PPV_ARGS(&object), nullptr)) || !object || !object->GetBufferSize()) {
        out.errors = "DXC returned no DXIL object";
        return out;
    }
    const auto* bytes = static_cast<const uint8_t*>(object->GetBufferPointer());
    out.dxil.assign(bytes, bytes + object->GetBufferSize());
    out.ok = true;
    return out;
}

// Every pass of one compile, in order; the first failure ends it and the later passes are "not_run". A failure is a
// record, never an exception (C9 U5c): a source that cannot be composed or a compiler object that cannot be created
// fails the compile with its status and code like a DXC error does.
struct Compiled {
    bool ok = true;
    const char* status = "compile_failed"; // the record status of a failure
    int32_t code = SA2_E_CHECK_FAILED; // and its status code
    Json records{Json::Array{}};
    std::vector<std::vector<uint8_t>> dxil;
    std::string error;
};
Compiled compile_passes(const Dxc& library, uint32_t kind, std::string_view text) {
    const CoreSources core{embedded_text(embedded::geometry, embedded::geometry_size),
                           embedded_text(embedded::draw, embedded::draw_size),
                           embedded_text(embedded::stages, embedded::stages_size)};
    Compiled out;
    std::string source;
    Json source_sha = nullptr;
    try {
        source = compose(kind, text, core);
        source_sha = sha256(source);
    } catch (const std::exception& error) {
        out.ok = false;
        out.error = std::string("composing the pass source failed: ") + error.what();
    }
    Json::Array records;
    for (const auto& pass : passes(kind)) {
        Json record{Json::Object{}};
        record["entry"] = pass.entry;
        record["profile"] = pass.profile;
        Json::Array argv;
        for (const auto& argument : arguments(pass)) argv.emplace_back(argument);
        record["arguments"] = argv;
        record["source_sha256"] = source_sha;
        record["dxil_sha256"] = nullptr;
        if (!out.ok) {
            record["status"] = "not_run";
        } else {
            try {
                auto result = run(library, source, pass);
                if (result.ok) {
                    record["status"] = "compiled";
                    record["dxil_sha256"] = sb::sha256(result.dxil);
                    out.dxil.push_back(std::move(result.dxil));
                } else {
                    record["status"] = "failed";
                    out.ok = false;
                    out.error = std::string(pass.entry) + " (" + pass.profile + "): " + result.errors;
                }
            } catch (const Failure& error) { // DxcCreateInstance(CLSID_DxcCompiler) for this pass
                record["status"] = "failed";
                out.ok = false;
                out.status = "compiler_unavailable";
                out.code = error.status;
                out.error = std::string(pass.entry) + " (" + pass.profile + "): " + error.what();
            } catch (const std::exception& error) {
                record["status"] = "failed";
                out.ok = false;
                out.error = std::string(pass.entry) + " (" + pass.profile + "): " + error.what();
            }
        }
        records.push_back(std::move(record));
    }
    out.records = Json(std::move(records));
    return out;
}

ComPtr<ID3D12RootSignature> stage_root(ID3D12Device* device) {
    D3D12_ROOT_PARAMETER params[23]{};
    params[0].ParameterType = D3D12_ROOT_PARAMETER_TYPE_CBV;
    params[0].Descriptor.ShaderRegister = 0;
    for (UINT i = 1; i <= 7; ++i) { params[i].ParameterType = D3D12_ROOT_PARAMETER_TYPE_SRV; params[i].Descriptor.ShaderRegister = i - 1; }
    for (UINT i = 8; i <= 9; ++i) { params[i].ParameterType = D3D12_ROOT_PARAMETER_TYPE_UAV; params[i].Descriptor.ShaderRegister = i - 8; }
    for (UINT i = 10; i < 18; ++i) { params[i].ParameterType = D3D12_ROOT_PARAMETER_TYPE_SRV; params[i].Descriptor.ShaderRegister = i - 3; }
    params[FrameConstants].ParameterType = D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS;
    params[FrameConstants].Constants = {1, 0, FrameConstantCount};
    const D3D12_DESCRIPTOR_RANGE arrays{D3D12_DESCRIPTOR_RANGE_TYPE_SRV, 2, 20, 0, D3D12_DESCRIPTOR_RANGE_OFFSET_APPEND};
    params[ArrayTable].ParameterType = D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE;
    params[ArrayTable].DescriptorTable = {1, &arrays};
    params[EvalRead].ParameterType = D3D12_ROOT_PARAMETER_TYPE_SRV;
    params[EvalRead].Descriptor.ShaderRegister = 22;
    params[EvalWrite].ParameterType = D3D12_ROOT_PARAMETER_TYPE_UAV;
    params[EvalWrite].Descriptor.ShaderRegister = 2;
    params[FrameWrite].ParameterType = D3D12_ROOT_PARAMETER_TYPE_UAV;
    params[FrameWrite].Descriptor.ShaderRegister = 3;
    D3D12_ROOT_SIGNATURE_DESC d{};
    d.NumParameters = 23;
    d.pParameters = params;
    d.Flags = D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT;
    ComPtr<ID3DBlob> blob, error;
    check(device, D3D12SerializeRootSignature(&d, D3D_ROOT_SIGNATURE_VERSION_1, &blob, &error), "D3D12SerializeRootSignature(stages)");
    ComPtr<ID3D12RootSignature> out;
    check(device, device->CreateRootSignature(0, blob->GetBufferPointer(), blob->GetBufferSize(), IID_PPV_ARGS(&out)), "CreateRootSignature(stages)");
    check(device, out->SetName(L"SA2 stage root signature"), "SetName(stages)");
    return out;
}

// The scene's draw pipeline state (scene.cpp graphics) with the stage root signature.
ComPtr<ID3D12PipelineState> graphics(ID3D12Device* device, ID3D12RootSignature* root, const std::vector<uint8_t>& vs,
                                     const std::vector<uint8_t>& ps, DXGI_FORMAT format, const wchar_t* label) {
    D3D12_GRAPHICS_PIPELINE_STATE_DESC d{}; d.pRootSignature = root; d.VS = {vs.data(), vs.size()}; d.PS = {ps.data(), ps.size()};
    auto& blend = d.BlendState.RenderTarget[0]; blend.SrcBlend = blend.SrcBlendAlpha = D3D12_BLEND_ONE;
    blend.DestBlend = blend.DestBlendAlpha = D3D12_BLEND_ZERO; blend.BlendOp = blend.BlendOpAlpha = D3D12_BLEND_OP_ADD;
    blend.LogicOp = D3D12_LOGIC_OP_NOOP; blend.RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL;
    d.SampleMask = UINT_MAX; d.RasterizerState.FillMode = D3D12_FILL_MODE_SOLID;
    d.RasterizerState.CullMode = D3D12_CULL_MODE_NONE; d.RasterizerState.DepthClipEnable = TRUE;
    d.DepthStencilState.DepthEnable = TRUE; d.DepthStencilState.DepthWriteMask = D3D12_DEPTH_WRITE_MASK_ALL;
    d.DepthStencilState.DepthFunc = D3D12_COMPARISON_FUNC_LESS_EQUAL;
    d.DepthStencilState.StencilReadMask = D3D12_DEFAULT_STENCIL_READ_MASK; d.DepthStencilState.StencilWriteMask = D3D12_DEFAULT_STENCIL_WRITE_MASK;
    D3D12_DEPTH_STENCILOP_DESC stencil{}; stencil.StencilFailOp = stencil.StencilDepthFailOp = stencil.StencilPassOp = D3D12_STENCIL_OP_KEEP;
    stencil.StencilFunc = D3D12_COMPARISON_FUNC_ALWAYS; d.DepthStencilState.FrontFace = d.DepthStencilState.BackFace = stencil;
    d.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE; d.NumRenderTargets = 1;
    d.RTVFormats[0] = format; d.DSVFormat = DXGI_FORMAT_D32_FLOAT; d.SampleDesc.Count = 1;
    ComPtr<ID3D12PipelineState> out;
    check(device, device->CreateGraphicsPipelineState(&d, IID_PPV_ARGS(&out)), "CreateGraphicsPipelineState(stages)");
    check(device, out->SetName(label), "SetName(stages)");
    return out;
}
ComPtr<ID3D12PipelineState> compute(ID3D12Device* device, ID3D12RootSignature* root, const std::vector<uint8_t>& cs, const wchar_t* label) {
    D3D12_COMPUTE_PIPELINE_STATE_DESC d{}; d.pRootSignature = root; d.CS = {cs.data(), cs.size()};
    ComPtr<ID3D12PipelineState> out;
    check(device, device->CreateComputePipelineState(&d, IID_PPV_ARGS(&out)), "CreateComputePipelineState(stages)");
    check(device, out->SetName(label), "SetName(stages)");
    return out;
}
constexpr DXGI_FORMAT Formats[2] = {DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_B8G8R8A8_UNORM};
}

int32_t Registry::compile(uint32_t kind, std::string_view text, uint64_t handle, Json& record) {
    record = Json(Json::Object{});
    record["format"] = RecordFormat;
    record["kind"] = kind_name(kind);
    record["status"] = "compiled";
    record["code"] = nullptr;
    record["handle"] = 0;
    record["stage_sha256"] = sha256(text);
    record["compiler"] = nullptr;
    record["passes"] = Json::Array{};
    record["core_passes"] = Json::Array{};
    record["error"] = "";
    auto fail = [&](const char* status, const std::string& error, int32_t code) {
        record["status"] = status;
        record["code"] = CompileFailed;
        record["error"] = utf8_clean(error);
        return code;
    };
    const Dxc* library = nullptr;
    try {
        library = &dxc();
    } catch (const Failure& error) {
        return fail("compiler_unavailable", error.what(), error.status);
    } catch (const std::exception& error) {
        return fail("compiler_unavailable", error.what(), SA2_E_UNAVAILABLE);
    }
    record["compiler"] = library->info;
    if (!core) {
        auto built = compile_passes(*library, CoreKind, {});
        record["core_passes"] = built.records;
        if (!built.ok) return fail(built.status, built.error, built.code);
        Core made;
        try {
            made.root = stage_root(device);
            made.eval = compute(device, made.root.Get(), built.dxil[0], L"SA2 stage eval (built-in vertex stage)");
            made.bound = compute(device, made.root.Get(), built.dxil[1], L"SA2 stage bound");
            made.decide = compute(device, made.root.Get(), built.dxil[2], L"SA2 stage decide");
            for (size_t i = 0; i < 2; ++i)
                made.draw[i] = graphics(device, made.root.Get(), built.dxil[3], built.dxil[4], Formats[i], L"SA2 stage draw (built-in fragment stage)");
        } catch (const Failure& error) {
            return fail("pipeline_failed", error.what(), error.status);
        }
        made.pass_vs = std::move(built.dxil[3]);
        core = std::move(made);
        core_passes = built.records;
    }
    record["core_passes"] = core_passes;
    auto built = compile_passes(*library, kind, text);
    record["passes"] = built.records;
    if (!built.ok) return fail(built.status, built.error, built.code);
    Stage stage;
    stage.kind = kind;
    try {
        if (kind == SA2_STAGE_VERTEX) {
            stage.eval = compute(device, core->root.Get(), built.dxil[0], L"SA2 stage eval (user vertex stage)");
        } else {
            for (size_t i = 0; i < 2; ++i)
                stage.draw[i] = graphics(device, core->root.Get(), core->pass_vs, built.dxil[0], Formats[i], L"SA2 stage draw (user fragment stage)");
        }
    } catch (const Failure& error) {
        return fail("pipeline_failed", error.what(), error.status);
    }
    stages[handle] = std::move(stage);
    record["handle"] = handle;
    return SA2_OK;
}

bool Registry::holds(uint64_t handle, uint32_t kind) const {
    const auto found = stages.find(handle);
    return found != stages.end() && (kind == 0 || found->second.kind == kind);
}

void Registry::release(uint64_t handle) {
    stages.erase(handle);
}

std::shared_ptr<const Chain> Registry::chain(uint64_t vertex, uint64_t fragment, uint32_t overdraw,
                                              std::vector<uint32_t> vertex_arrays, std::vector<uint32_t> fragment_arrays) const {
    scene_require(core.has_value(), "no stage was compiled on this device", SA2_E_INVALID_ARGUMENT);
    auto chain = std::make_shared<Chain>();
    chain->root = core->root;
    chain->bound = core->bound;
    chain->decide = core->decide;
    chain->eval = vertex ? stages.at(vertex).eval : core->eval;
    chain->draw = fragment ? stages.at(fragment).draw : core->draw;
    chain->vertex = vertex;
    chain->fragment = fragment;
    chain->overdraw = overdraw;
    chain->vertex_arrays = std::move(vertex_arrays);
    chain->fragment_arrays = std::move(fragment_arrays);
    return chain;
}
}
