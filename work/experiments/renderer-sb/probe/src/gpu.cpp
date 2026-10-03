#include "probe.h"
#include <windows.h>
#include <d3d12.h>
#include <dxgi1_6.h>
#include <wrl/client.h>
#include <algorithm>
#include <cstring>
#include <iostream>
#include <limits>
#include <optional>

namespace sb {
using Microsoft::WRL::ComPtr;
static void hr(HRESULT result,const char* action){
    if(FAILED(result)){std::ostringstream s;s<<action<<" failed (0x"<<std::hex<<uint32_t(result)<<')';throw std::runtime_error(s.str());}
}
static int64_t qpc(){LARGE_INTEGER t{};require(QueryPerformanceCounter(&t)!=0,"QPC unavailable");return t.QuadPart;}
static std::string utf8(const wchar_t* text){int n=WideCharToMultiByte(CP_UTF8,0,text,-1,nullptr,0,nullptr,nullptr);require(n>0,"UTF-8 conversion failed");std::string out(size_t(n),'\0');WideCharToMultiByte(CP_UTF8,0,text,-1,out.data(),n,nullptr,nullptr);out.pop_back();return out;}
static LRESULT CALLBACK windowProc(HWND hwnd,UINT message,WPARAM w,LPARAM l){
    if(message==WM_DESTROY){PostQuitMessage(0);return 0;}
    if(message==WM_KEYDOWN&&w==VK_ESCAPE){DestroyWindow(hwnd);return 0;}
    return DefWindowProcW(hwnd,message,w,l);
}
struct Constants {
    Matrix q;
    std::array<float,4> normal;
    float radius,cs,ss,d4,zoom,aspect,theta;
    uint32_t sampleCount;
    std::array<float,4> planeU,planeV;
};
static_assert(sizeof(Constants)==144);
static Constants constants(const Assets& a,const Matrix& q,float aspect,float theta,uint32_t count=0){return {q,a.normal,a.radius,0.76f,0.82f,1.18f,1.15f,aspect,theta,count,a.planeU,a.planeV};}
struct Buffer {
    ComPtr<ID3D12Resource> resource;
    D3D12_RESOURCE_STATES state=D3D12_RESOURCE_STATE_COMMON;
    uint64_t size=0;
};
struct Frame {
    ComPtr<ID3D12CommandAllocator> allocator;
    Buffer cb,upload;
    uint8_t *cbData=nullptr,*uploadData=nullptr;
    uint64_t fence=0;
};
class Gpu {
public:
    const Options& opt;
    ComPtr<IDXGIFactory6> factory;
    ComPtr<IDXGIAdapter1> adapter;
    ComPtr<IDXGIAdapter3> memoryAdapter;
    ComPtr<ID3D12Device> device;
    ComPtr<ID3D12CommandQueue> queue;
    ComPtr<ID3D12GraphicsCommandList> list;
    ComPtr<ID3D12Fence> fence;
    ComPtr<ID3D12RootSignature> root;
    ComPtr<ID3D12PipelineState> drawPso,countPso,computePso;
    ComPtr<IDXGISwapChain3> swap;
    ComPtr<ID3D12DescriptorHeap> rtvHeap,dsvHeap;
    std::array<ComPtr<ID3D12Resource>,3> targets;
    ComPtr<ID3D12Resource> depth,msaaTarget;
    std::array<Frame,2> frame;
    std::array<Buffer,6> geometry; // vertices, ids, centers, frames, animated flags, samples
    std::array<Buffer,3> labels;
    std::array<uint64_t,3> labelRevision{};
    Buffer readback;
    HANDLE event=nullptr;
    HWND hwnd=nullptr;
    uint64_t nextFence=1,readbackSlots=0;
    UINT width=2560,height=1600,refresh=60,rtvStep=0;
    int64_t frequency=0;
    bool tearing=false;
    double vramPeak=0;
    uint64_t presents=0,occluded=0; // occluded: Present returned DXGI_STATUS_OCCLUDED
    bool foregroundAtStart=false;
    std::string adapterName,driver;
    Frame* active=nullptr;

    Gpu(const Assets& a,const Options& options,const std::vector<uint32_t>& sample):opt(options){
        LARGE_INTEGER f{};require(QueryPerformanceFrequency(&f)!=0,"QPC frequency unavailable");frequency=f.QuadPart;
        UINT factoryFlags=0;
        if(opt.debug){ComPtr<ID3D12Debug> debug;hr(D3D12GetDebugInterface(IID_PPV_ARGS(&debug)),"D3D12 debug layer (install the SDK layer first)");debug->EnableDebugLayer();factoryFlags=DXGI_CREATE_FACTORY_DEBUG;}
        hr(CreateDXGIFactory2(factoryFlags,IID_PPV_ARGS(&factory)),"CreateDXGIFactory2");
        if(opt.warp)hr(factory->EnumWarpAdapter(IID_PPV_ARGS(&adapter)),"EnumWarpAdapter");
        else hr(factory->EnumAdapterByGpuPreference(0,DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE,IID_PPV_ARGS(&adapter)),"high-performance adapter enumeration");
        DXGI_ADAPTER_DESC1 desc{};hr(adapter->GetDesc1(&desc),"adapter description");
        require(opt.warp||!(desc.Flags&DXGI_ADAPTER_FLAG_SOFTWARE),"high-performance adapter is software; use --warp only for debugging");adapterName=utf8(desc.Description);
        LARGE_INTEGER version{};auto versionResult=adapter->CheckInterfaceSupport(__uuidof(IDXGIDevice),&version);
        if(SUCCEEDED(versionResult)){uint64_t v=uint64_t(version.QuadPart);driver=std::to_string((v>>48)&65535)+'.'+std::to_string((v>>32)&65535)+'.'+std::to_string((v>>16)&65535)+'.'+std::to_string(v&65535);}
        else {std::ostringstream s;s<<"UMD version unavailable (0x"<<std::hex<<uint32_t(versionResult)<<')';driver=s.str();}
        hr(adapter.As(&memoryAdapter),"IDXGIAdapter3");
        hr(D3D12CreateDevice(adapter.Get(),D3D_FEATURE_LEVEL_12_0,IID_PPV_ARGS(&device)),"D3D12CreateDevice");
        D3D12_COMMAND_QUEUE_DESC q{};q.Type=D3D12_COMMAND_LIST_TYPE_DIRECT;hr(device->CreateCommandQueue(&q,IID_PPV_ARGS(&queue)),"CreateCommandQueue");
        hr(device->CreateFence(0,D3D12_FENCE_FLAG_NONE,IID_PPV_ARGS(&fence)),"CreateFence");
        event=CreateEventW(nullptr,FALSE,FALSE,nullptr);require(event!=nullptr,"cannot create fence event");
        for(auto& fctx:frame){
            hr(device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT,IID_PPV_ARGS(&fctx.allocator)),"CreateCommandAllocator");
            fctx.cb=buffer(256,D3D12_HEAP_TYPE_UPLOAD,D3D12_RESOURCE_STATE_GENERIC_READ);
            fctx.upload=buffer(LabelBytes,D3D12_HEAP_TYPE_UPLOAD,D3D12_RESOURCE_STATE_GENERIC_READ);
            D3D12_RANGE none{0,0};hr(fctx.cb.resource->Map(0,&none,reinterpret_cast<void**>(&fctx.cbData)),"map constants");hr(fctx.upload.resource->Map(0,&none,reinterpret_cast<void**>(&fctx.uploadData)),"map label upload ring");
        }
        hr(device->CreateCommandList(0,D3D12_COMMAND_LIST_TYPE_DIRECT,frame[0].allocator.Get(),nullptr,IID_PPV_ARGS(&list)),"CreateCommandList");hr(list->Close(),"initial Close");
        makeRoot();
        begin(0);
        std::vector<Buffer> uploads;
        auto put=[&](auto& values){return staticBuffer(values.data(),values.size()*sizeof(values[0]),uploads);};
        geometry[0]=put(a.vertices);geometry[1]=put(a.local);geometry[2]=put(a.centers);geometry[3]=put(a.frames);geometry[4]=put(a.flags);geometry[5]=put(sample);
        for(auto& label:labels)label=put(a.oracle[0]);
        submit(false);idle(); // The static upload resources can now be released.
        makeTargets();
        drawPso=graphics("draw_vs.dxil");
        if(opt.geometry){countPso=graphics("count_vs.dxil");auto code=bytes(exeDir()/"geometry_cs.dxil");D3D12_COMPUTE_PIPELINE_STATE_DESC p{};p.pRootSignature=root.Get();p.CS={code.data(),code.size()};hr(device->CreateComputePipelineState(&p,IID_PPV_ARGS(&computePso)),"compute pipeline");}
        else if(opt.scene=="w3"||opt.scene=="w4"){
            double slots=std::ceil(opt.duration*1000/opt.turnMs)+2;
            require(slots<double(std::numeric_limits<uint64_t>::max()/LabelBytes),"label readback size overflow");readbackSlots=uint64_t(slots);
            try{readback=buffer(readbackSlots*LabelBytes,D3D12_HEAP_TYPE_READBACK,D3D12_RESOURCE_STATE_COPY_DEST);}
            catch(const std::exception&){throw std::runtime_error("cannot allocate the run's preserved label readback buffer ("+std::to_string(readbackSlots*LabelBytes/1048576)+" MiB); no timing run started");}
        }
    }
    ~Gpu(){
        if(queue&&fence&&event){try{idle();}catch(...){}}
        for(auto& f:frame){if(f.cbData)f.cb.resource->Unmap(0,nullptr);if(f.uploadData)f.upload.resource->Unmap(0,nullptr);}
        if(hwnd&&IsWindow(hwnd))DestroyWindow(hwnd);
        if(event)CloseHandle(event);
    }
    Buffer buffer(uint64_t size,D3D12_HEAP_TYPE type,D3D12_RESOURCE_STATES state,D3D12_RESOURCE_FLAGS flags=D3D12_RESOURCE_FLAG_NONE){
        require(size>0,"zero-sized GPU buffer");Buffer out;out.size=size;out.state=state;
        D3D12_HEAP_PROPERTIES heap{};heap.Type=type;heap.CreationNodeMask=heap.VisibleNodeMask=1;
        D3D12_RESOURCE_DESC d{};d.Dimension=D3D12_RESOURCE_DIMENSION_BUFFER;d.Width=size;d.Height=1;d.DepthOrArraySize=1;d.MipLevels=1;d.SampleDesc.Count=1;d.Layout=D3D12_TEXTURE_LAYOUT_ROW_MAJOR;d.Flags=flags;
        hr(device->CreateCommittedResource(&heap,D3D12_HEAP_FLAG_NONE,&d,state,nullptr,IID_PPV_ARGS(&out.resource)),"CreateCommittedResource buffer");return out;
    }
    void transition(ID3D12Resource* resource,D3D12_RESOURCE_STATES before,D3D12_RESOURCE_STATES after){
        if(before==after)return;D3D12_RESOURCE_BARRIER b{};b.Type=D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;b.Transition.pResource=resource;b.Transition.Subresource=D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;b.Transition.StateBefore=before;b.Transition.StateAfter=after;list->ResourceBarrier(1,&b);
    }
    void transition(Buffer& b,D3D12_RESOURCE_STATES after){transition(b.resource.Get(),b.state,after);b.state=after;}
    Buffer staticBuffer(const void* data,uint64_t size,std::vector<Buffer>& uploads){
        auto target=buffer(size,D3D12_HEAP_TYPE_DEFAULT,D3D12_RESOURCE_STATE_COPY_DEST);auto up=buffer(size,D3D12_HEAP_TYPE_UPLOAD,D3D12_RESOURCE_STATE_GENERIC_READ);
        void* mapped=nullptr;D3D12_RANGE none{0,0};hr(up.resource->Map(0,&none,&mapped),"map static upload");std::memcpy(mapped,data,size_t(size));up.resource->Unmap(0,nullptr);
        list->CopyBufferRegion(target.resource.Get(),0,up.resource.Get(),0,size);transition(target,D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);uploads.push_back(std::move(up));return target;
    }
    void wait(uint64_t value){
        if(fence->GetCompletedValue()>=value)return;
        hr(fence->SetEventOnCompletion(value,event),"SetEventOnCompletion");require(WaitForSingleObject(event,INFINITE)==WAIT_OBJECT_0,"fence wait failed");
        hr(device->GetDeviceRemovedReason(),"GPU device removed during fence wait");
    }
    void idle(){auto value=nextFence++;hr(queue->Signal(fence.Get(),value),"idle Signal");wait(value);}
    void begin(uint64_t serial){
        // Reusing two frame contexts permits at most two submissions in flight.
        active=&frame[serial%frame.size()];wait(active->fence);hr(active->allocator->Reset(),"allocator Reset");hr(list->Reset(active->allocator.Get(),nullptr),"command list Reset");
    }
    void submit(bool present){
        hr(list->Close(),"command list Close");ID3D12CommandList* submitted[]={list.Get()};queue->ExecuteCommandLists(1,submitted);
        if(present){auto result=swap->Present(opt.vsync?1:0,!opt.vsync&&tearing?DXGI_PRESENT_ALLOW_TEARING:0);hr(result,"Present");++presents;if(result==DXGI_STATUS_OCCLUDED)++occluded;}
        active->fence=nextFence++;hr(queue->Signal(fence.Get(),active->fence),"frame Signal");
    }
    void makeRoot(){
        D3D12_ROOT_PARAMETER params[10]{};params[0].ParameterType=D3D12_ROOT_PARAMETER_TYPE_CBV;params[0].Descriptor.ShaderRegister=0;
        for(UINT i=1;i<=7;++i){params[i].ParameterType=D3D12_ROOT_PARAMETER_TYPE_SRV;params[i].Descriptor.ShaderRegister=i-1;}
        for(UINT i=8;i<=9;++i){params[i].ParameterType=D3D12_ROOT_PARAMETER_TYPE_UAV;params[i].Descriptor.ShaderRegister=i-8;}
        D3D12_ROOT_SIGNATURE_DESC d{};d.NumParameters=10;d.pParameters=params;d.Flags=D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT;
        ComPtr<ID3DBlob> blob,error;auto result=D3D12SerializeRootSignature(&d,D3D_ROOT_SIGNATURE_VERSION_1,&blob,&error);
        if(FAILED(result)&&error)throw std::runtime_error(std::string(static_cast<const char*>(error->GetBufferPointer()),error->GetBufferSize()));hr(result,"serialize root signature");
        hr(device->CreateRootSignature(0,blob->GetBufferPointer(),blob->GetBufferSize(),IID_PPV_ARGS(&root)),"CreateRootSignature");
    }
    ComPtr<ID3D12Resource> texture(DXGI_FORMAT format,D3D12_RESOURCE_FLAGS flags,D3D12_RESOURCE_STATES state,unsigned samples,const D3D12_CLEAR_VALUE* clear){
        D3D12_HEAP_PROPERTIES heap{};heap.Type=D3D12_HEAP_TYPE_DEFAULT;heap.CreationNodeMask=heap.VisibleNodeMask=1;
        D3D12_RESOURCE_DESC d{};d.Dimension=D3D12_RESOURCE_DIMENSION_TEXTURE2D;d.Width=width;d.Height=height;d.DepthOrArraySize=1;d.MipLevels=1;d.Format=format;d.SampleDesc.Count=samples;d.Flags=flags;
        ComPtr<ID3D12Resource> target;hr(device->CreateCommittedResource(&heap,D3D12_HEAP_FLAG_NONE,&d,state,clear,IID_PPV_ARGS(&target)),"CreateCommittedResource texture");return target;
    }
    void makeTargets(){
        if(opt.msaa==4)for(auto format:{DXGI_FORMAT_R8G8B8A8_UNORM,DXGI_FORMAT_D32_FLOAT}){D3D12_FEATURE_DATA_MULTISAMPLE_QUALITY_LEVELS m{};m.Format=format;m.SampleCount=4;hr(device->CheckFeatureSupport(D3D12_FEATURE_MULTISAMPLE_QUALITY_LEVELS,&m,sizeof(m)),"MSAA support query");require(m.NumQualityLevels>0,"MSAA 4x is unsupported on this adapter");}
        if(!opt.geometry){
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            POINT origin{0,0};HMONITOR monitor=MonitorFromPoint(origin,MONITOR_DEFAULTTOPRIMARY);MONITORINFOEXW info{};info.cbSize=sizeof(info);require(GetMonitorInfoW(monitor,&info)!=0,"primary monitor information unavailable");
            DEVMODEW mode{};mode.dmSize=sizeof(mode);require(EnumDisplaySettingsW(info.szDevice,ENUM_CURRENT_SETTINGS,&mode)!=0,"current monitor mode unavailable");width=mode.dmPelsWidth;height=mode.dmPelsHeight;refresh=mode.dmDisplayFrequency;
            WNDCLASSW wc{};wc.lpfnWndProc=windowProc;wc.hInstance=GetModuleHandleW(nullptr);wc.lpszClassName=L"Magic600SBProbe";wc.hCursor=LoadCursorW(nullptr,IDC_ARROW);require(RegisterClassW(&wc)!=0,"RegisterClass failed");
            hwnd=CreateWindowExW(WS_EX_TOPMOST,wc.lpszClassName,L"Magic 600 Cell S-B Direct3D 12 probe",WS_POPUP|WS_VISIBLE,info.rcMonitor.left,info.rcMonitor.top,int(width),int(height),nullptr,nullptr,wc.hInstance,nullptr);require(hwnd!=nullptr,"CreateWindow failed");
            // Topmost so no other window covers the probe; the foreground request may be refused
            // when the launching process is not in the foreground, which run.json records.
            SetForegroundWindow(hwnd);
            BOOL allowed=FALSE;tearing=SUCCEEDED(factory->CheckFeatureSupport(DXGI_FEATURE_PRESENT_ALLOW_TEARING,&allowed,sizeof(allowed)))&&allowed&&!opt.vsync;
            DXGI_SWAP_CHAIN_DESC1 s{};s.Width=width;s.Height=height;s.Format=DXGI_FORMAT_R8G8B8A8_UNORM;s.SampleDesc.Count=1;s.BufferUsage=DXGI_USAGE_RENDER_TARGET_OUTPUT;s.BufferCount=3;s.SwapEffect=DXGI_SWAP_EFFECT_FLIP_DISCARD;s.Flags=tearing?DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING:0;
            ComPtr<IDXGISwapChain1> base;hr(factory->CreateSwapChainForHwnd(queue.Get(),hwnd,&s,nullptr,nullptr,&base),"CreateSwapChainForHwnd");hr(base.As(&swap),"IDXGISwapChain3");hr(factory->MakeWindowAssociation(hwnd,DXGI_MWA_NO_ALT_ENTER),"MakeWindowAssociation");for(UINT i=0;i<3;++i)hr(swap->GetBuffer(i,IID_PPV_ARGS(&targets[i])),"swap-chain buffer");
        }else targets[0]=texture(DXGI_FORMAT_R8G8B8A8_UNORM,D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET,D3D12_RESOURCE_STATE_RENDER_TARGET,1,nullptr);
        D3D12_DESCRIPTOR_HEAP_DESC rh{};rh.Type=D3D12_DESCRIPTOR_HEAP_TYPE_RTV;rh.NumDescriptors=4;hr(device->CreateDescriptorHeap(&rh,IID_PPV_ARGS(&rtvHeap)),"RTV heap");rtvStep=device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
        for(UINT i=0;i<(opt.geometry?1u:3u);++i)device->CreateRenderTargetView(targets[i].Get(),nullptr,rtv(i));
        D3D12_DESCRIPTOR_HEAP_DESC dh{};dh.Type=D3D12_DESCRIPTOR_HEAP_TYPE_DSV;dh.NumDescriptors=1;hr(device->CreateDescriptorHeap(&dh,IID_PPV_ARGS(&dsvHeap)),"DSV heap");D3D12_CLEAR_VALUE dc{};dc.Format=DXGI_FORMAT_D32_FLOAT;dc.DepthStencil.Depth=1;
        depth=texture(DXGI_FORMAT_D32_FLOAT,D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL,D3D12_RESOURCE_STATE_DEPTH_WRITE,opt.msaa,&dc);device->CreateDepthStencilView(depth.Get(),nullptr,dsvHeap->GetCPUDescriptorHandleForHeapStart());
        if(opt.msaa==4){D3D12_CLEAR_VALUE cc{};cc.Format=DXGI_FORMAT_R8G8B8A8_UNORM;cc.Color[0]=0.13f;cc.Color[1]=0.145f;cc.Color[2]=0.16f;cc.Color[3]=1;msaaTarget=texture(cc.Format,D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET,D3D12_RESOURCE_STATE_RENDER_TARGET,4,&cc);device->CreateRenderTargetView(msaaTarget.Get(),nullptr,rtv(3));}
    }
    D3D12_CPU_DESCRIPTOR_HANDLE rtv(UINT i){auto h=rtvHeap->GetCPUDescriptorHandleForHeapStart();h.ptr+=SIZE_T(i)*rtvStep;return h;}
    ComPtr<ID3D12PipelineState> graphics(const char* shader){
        auto vs=bytes(exeDir()/shader),ps=bytes(exeDir()/"draw_ps.dxil");D3D12_GRAPHICS_PIPELINE_STATE_DESC d{};d.pRootSignature=root.Get();d.VS={vs.data(),vs.size()};d.PS={ps.data(),ps.size()};
        auto& blend=d.BlendState.RenderTarget[0];blend.SrcBlend=D3D12_BLEND_ONE;blend.DestBlend=D3D12_BLEND_ZERO;blend.BlendOp=D3D12_BLEND_OP_ADD;blend.SrcBlendAlpha=D3D12_BLEND_ONE;blend.DestBlendAlpha=D3D12_BLEND_ZERO;blend.BlendOpAlpha=D3D12_BLEND_OP_ADD;blend.LogicOp=D3D12_LOGIC_OP_NOOP;blend.RenderTargetWriteMask=D3D12_COLOR_WRITE_ENABLE_ALL;
        d.SampleMask=UINT_MAX;d.RasterizerState.FillMode=D3D12_FILL_MODE_SOLID;d.RasterizerState.CullMode=D3D12_CULL_MODE_NONE;d.RasterizerState.DepthClipEnable=TRUE;d.RasterizerState.MultisampleEnable=opt.msaa==4;
        d.DepthStencilState.DepthEnable=TRUE;d.DepthStencilState.DepthWriteMask=D3D12_DEPTH_WRITE_MASK_ALL;d.DepthStencilState.DepthFunc=D3D12_COMPARISON_FUNC_LESS_EQUAL;
        d.DepthStencilState.StencilReadMask=D3D12_DEFAULT_STENCIL_READ_MASK;d.DepthStencilState.StencilWriteMask=D3D12_DEFAULT_STENCIL_WRITE_MASK;
        D3D12_DEPTH_STENCILOP_DESC stencil{};stencil.StencilFailOp=stencil.StencilDepthFailOp=stencil.StencilPassOp=D3D12_STENCIL_OP_KEEP;stencil.StencilFunc=D3D12_COMPARISON_FUNC_ALWAYS;d.DepthStencilState.FrontFace=d.DepthStencilState.BackFace=stencil;
        d.PrimitiveTopologyType=D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;d.NumRenderTargets=1;d.RTVFormats[0]=DXGI_FORMAT_R8G8B8A8_UNORM;d.DSVFormat=DXGI_FORMAT_D32_FLOAT;d.SampleDesc.Count=opt.msaa;
        ComPtr<ID3D12PipelineState> p;hr(device->CreateGraphicsPipelineState(&d,IID_PPV_ARGS(&p)),"graphics pipeline");return p;
    }
    void bind(const Constants& c,unsigned labelIndex,bool compute){
        std::memcpy(active->cbData,&c,sizeof(c));
        if(compute)list->SetComputeRootSignature(root.Get());else list->SetGraphicsRootSignature(root.Get());
        auto cb=active->cb.resource->GetGPUVirtualAddress();if(compute)list->SetComputeRootConstantBufferView(0,cb);else list->SetGraphicsRootConstantBufferView(0,cb);
        for(UINT i=0;i<7;++i){auto address=(i<5?geometry[i]:i==5?labels[labelIndex]:geometry[5]).resource->GetGPUVirtualAddress();if(compute)list->SetComputeRootShaderResourceView(i+1,address);else list->SetGraphicsRootShaderResourceView(i+1,address);}
    }
    void uploadLabels(unsigned index,const std::vector<uint32_t>& values){
        std::memcpy(active->uploadData,values.data(),size_t(LabelBytes));transition(labels[index],D3D12_RESOURCE_STATE_COPY_DEST);list->CopyBufferRegion(labels[index].resource.Get(),0,active->upload.resource.Get(),0,LabelBytes);transition(labels[index],D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
    }
    void draw(const Constants& c,unsigned labelIndex,ID3D12PipelineState* pipeline,Buffer* counter=nullptr){
        UINT back=opt.geometry?0:swap->GetCurrentBackBufferIndex();
        if(!opt.geometry)transition(targets[back].Get(),D3D12_RESOURCE_STATE_PRESENT,opt.msaa==4?D3D12_RESOURCE_STATE_RESOLVE_DEST:D3D12_RESOURCE_STATE_RENDER_TARGET);
        list->SetPipelineState(pipeline);bind(c,labelIndex,false);
        if(counter)list->SetGraphicsRootUnorderedAccessView(8,counter->resource->GetGPUVirtualAddress());
        D3D12_VIEWPORT viewport{0,0,float(width),float(height),0,1};D3D12_RECT rect{0,0,LONG(width),LONG(height)};list->RSSetViewports(1,&viewport);list->RSSetScissorRects(1,&rect);
        auto target=rtv(opt.msaa==4?3:back);auto dsv=dsvHeap->GetCPUDescriptorHandleForHeapStart();list->OMSetRenderTargets(1,&target,FALSE,&dsv);
        const float colour[4]={0.13f,0.145f,0.16f,1};list->ClearRenderTargetView(target,colour,0,nullptr);list->ClearDepthStencilView(dsv,D3D12_CLEAR_FLAG_DEPTH,1,0,0,nullptr);list->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        list->DrawInstanced(Vertices,Cells,0,0);
        if(opt.msaa==4){
            transition(msaaTarget.Get(),D3D12_RESOURCE_STATE_RENDER_TARGET,D3D12_RESOURCE_STATE_RESOLVE_SOURCE);
            if(opt.geometry)transition(targets[0].Get(),D3D12_RESOURCE_STATE_RENDER_TARGET,D3D12_RESOURCE_STATE_RESOLVE_DEST);
            list->ResolveSubresource(targets[back].Get(),0,msaaTarget.Get(),0,DXGI_FORMAT_R8G8B8A8_UNORM);transition(msaaTarget.Get(),D3D12_RESOURCE_STATE_RESOLVE_SOURCE,D3D12_RESOURCE_STATE_RENDER_TARGET);
            if(opt.geometry)transition(targets[0].Get(),D3D12_RESOURCE_STATE_RESOLVE_DEST,D3D12_RESOURCE_STATE_RENDER_TARGET);
        }
        if(!opt.geometry)transition(targets[back].Get(),opt.msaa==4?D3D12_RESOURCE_STATE_RESOLVE_DEST:D3D12_RESOURCE_STATE_RENDER_TARGET,D3D12_RESOURCE_STATE_PRESENT);
    }
    void copyLabels(unsigned actuallyBound,uint64_t slot){
        require(slot<readbackSlots,"preserved label readback capacity exceeded");auto& bound=labels[actuallyBound];transition(bound,D3D12_RESOURCE_STATE_COPY_SOURCE);
        list->CopyBufferRegion(readback.resource.Get(),slot*LabelBytes,bound.resource.Get(),0,LabelBytes);transition(bound,D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
    }
    void sampleMemory(){DXGI_QUERY_VIDEO_MEMORY_INFO info{};hr(memoryAdapter->QueryVideoMemoryInfo(0,DXGI_MEMORY_SEGMENT_GROUP_LOCAL,&info),"QueryVideoMemoryInfo");vramPeak=std::max(vramPeak,double(info.CurrentUsage)/1048576);}
    Json environment(){
        SYSTEM_POWER_STATUS power{};require(GetSystemPowerStatus(&power)!=0,"power source unavailable");std::string source=power.ACLineStatus==1?"mains":power.ACLineStatus==0?"battery":"unknown";
        MONITORINFOEXW info{};info.cbSize=sizeof(info);require(GetMonitorInfoW(MonitorFromWindow(hwnd,MONITOR_DEFAULTTOPRIMARY),&info)!=0,"window monitor unavailable");DEVMODEW mode{};mode.dmSize=sizeof(mode);require(EnumDisplaySettingsW(info.szDevice,ENUM_CURRENT_SETTINGS,&mode)!=0,"window monitor mode unavailable");
        return Json::Object{{"power_source",source},{"declared",opt.declared},{"display",Json::Object{{"width",mode.dmPelsWidth},{"height",mode.dmPelsHeight}}},{"backbuffer",Json::Object{{"width",width},{"height",height}}},{"refresh_hz",mode.dmDisplayFrequency},{"vsync",opt.vsync},{"tearing",tearing},{"adapter",adapterName},{"driver",driver},{"msaa",opt.msaa},{"warp",opt.warp}};
    }
};

static int geometryCheck(const Assets& a,const Options& opt){
    auto sample=a.samples();auto reference=repoRoot()/"work/experiments/renderer-sb/reference";bool missing=false,failed=false;
    auto samplePath=reference/"sample.u32";
    if(fs::exists(samplePath)){auto b=bytes(samplePath);require(b.size()==sample.size()*4,"geometry sample length mismatch");std::vector<uint32_t> supplied(sample.size());std::memcpy(supplied.data(),b.data(),b.size());require(supplied==sample,"geometry sample differs from SPEC rule");}else missing=true;
    if(fs::exists(reference/"index.json")){
        // The parallel reference packet owns the exact index layout. Its file digest
        // map is checked when present; the sample rule and sizes are always checked.
        auto index=readJson(reference/"index.json");auto it=index.object().find("files");
        if(it!=index.object().end())for(const auto& [name,entry]:it->second.object()){
            auto path=reference/fs::path(name).filename();if(!fs::exists(path)){missing=true;continue;}
            const auto& digest=std::holds_alternative<std::string>(entry.value)?entry:entry.at("sha256");require(sha256(bytes(path))==digest.string(),"geometry reference SHA-256 mismatch");
        }
    }else missing=true;
    Gpu gpu(a,opt,sample);
    auto result=gpu.buffer(sample.size()*12,D3D12_HEAP_TYPE_DEFAULT,D3D12_RESOURCE_STATE_UNORDERED_ACCESS,D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
    auto resultRead=gpu.buffer(sample.size()*12,D3D12_HEAP_TYPE_READBACK,D3D12_RESOURCE_STATE_COPY_DEST);
    auto counter=gpu.buffer(Cells*4,D3D12_HEAP_TYPE_DEFAULT,D3D12_RESOURCE_STATE_COPY_DEST,D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
    auto counterRead=gpu.buffer(Cells*4,D3D12_HEAP_TYPE_READBACK,D3D12_RESOURCE_STATE_COPY_DEST);
    auto zero=gpu.buffer(Cells*4,D3D12_HEAP_TYPE_UPLOAD,D3D12_RESOURCE_STATE_GENERIC_READ);
    void* mapped=nullptr;D3D12_RANGE none{0,0};hr(zero.resource->Map(0,&none,&mapped),"map counter reset");std::memset(mapped,0,Cells*4);zero.resource->Unmap(0,nullptr);
    gpu.begin(0);gpu.list->CopyBufferRegion(counter.resource.Get(),0,zero.resource.Get(),0,Cells*4);gpu.transition(counter,D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
    gpu.draw(constants(a,identity(),1.6f,0),0,gpu.countPso.Get(),&counter);gpu.transition(counter,D3D12_RESOURCE_STATE_COPY_SOURCE);gpu.list->CopyBufferRegion(counterRead.resource.Get(),0,counter.resource.Get(),0,Cells*4);gpu.submit(false);gpu.idle();
    D3D12_RANGE countRange{0,Cells*4};hr(counterRead.resource->Map(0,&countRange,&mapped),"map draw invocation counts");auto counts=static_cast<const uint32_t*>(mapped);Json::Array countArray;uint64_t countFailures=0;for(UINT cell=0;cell<Cells;++cell){countArray.emplace_back(counts[cell]);if(counts[cell]!=Vertices)++countFailures;}counterRead.resource->Unmap(0,&none);failed=countFailures>0;
    Json::Array results;
    for(const auto& camera:a.cameras.at("cameras").array()){
        Matrix q=identity();for(const auto& r:camera.at("rotations").array())rotate(q,int(r.array()[0].integer()),int(r.array()[1].integer()),r.array()[2].number());
        const std::array<std::pair<const char*,double>,3> poses={{{"start",0},{"mid",a.angle*0.5},{"end",a.angle}}};
        for(const auto& [pose,theta]:poses){
            gpu.begin(0);gpu.list->SetPipelineState(gpu.computePso.Get());gpu.bind(constants(a,q,1.6f,float(theta),uint32_t(sample.size())),0,true);gpu.list->SetComputeRootUnorderedAccessView(9,result.resource->GetGPUVirtualAddress());gpu.list->Dispatch(UINT((sample.size()+63)/64),1,1);
            gpu.transition(result,D3D12_RESOURCE_STATE_COPY_SOURCE);gpu.list->CopyBufferRegion(resultRead.resource.Get(),0,result.resource.Get(),0,result.size);gpu.transition(result,D3D12_RESOURCE_STATE_UNORDERED_ACCESS);gpu.submit(false);gpu.idle();
            std::string name=camera.at("name").string()+'_'+pose+".f32";auto path=reference/name;double maxError=0;uint64_t failures=0;bool available=fs::exists(path);
            D3D12_RANGE range{0,SIZE_T(result.size)};hr(resultRead.resource->Map(0,&range,&mapped),"map projected samples");auto values=static_cast<const float*>(mapped);
            std::vector<float> ref;
            if(available){auto raw=bytes(path);require(raw.size()==result.size,"reference projection length mismatch");ref.resize(raw.size()/4);std::memcpy(ref.data(),raw.data(),raw.size());}else missing=true;
            for(size_t i=0;i<sample.size()*3;++i){if(!std::isfinite(values[i])){++failures;continue;}if(available){double error=std::abs(double(values[i])-ref[i]);if(!std::isfinite(ref[i])||error>1e-4+1e-4*std::abs(double(ref[i])))++failures;maxError=std::max(maxError,error);}}
            resultRead.resource->Unmap(0,&none);failed|=failures>0;
            results.emplace_back(Json::Object{{"camera",camera.at("name")},{"pose",pose},{"samples",uint64_t(sample.size())},{"max_abs_error",available?Json(maxError):Json()},{"failures",failures},{"status",!available?"reference-missing":failures?"fail":"pass"}});
        }
    }
    std::string status=failed?"fail":missing?"reference-missing":"pass";
    Json output=Json::Object{{"format","magic600-sb-geometry-check-v1"},{"status",status},{"build_identity",buildIdentity()},{"adapter",gpu.adapterName},{"results",results},
        {"per_cell_count",Json::Object{{"cells",Cells},{"expected",Vertices},{"failures",countFailures},{"counts",countArray},{"status",countFailures?"fail":"pass"}}}};
    fs::create_directories(opt.out);writeText(opt.out/"geometry_check.json",output.dump()+'\n');std::cout<<"geometry check: "<<(status=="reference-missing"?"reference missing":status)<<'\n';return status=="pass"?0:1;
}
static void selfTiming(const std::vector<Trace>& trace,int64_t frequency,int64_t start,int64_t stop){
    int64_t begin=start,end=stop;if(stop-start>=190*frequency){begin=start+10*frequency;end=start+190*frequency;}
    std::vector<double> steps;uint64_t frames=0;
    for(size_t i=0;i<trace.size();++i)if(trace[i].qpc>=begin&&trace[i].qpc<end){++frames;if(i>0)steps.push_back(double(trace[i].qpc-trace[i-1].qpc)*1000/double(frequency));}
    double fps=0,p99=0;if(!steps.empty()){double total=0;for(double step:steps)total+=step;fps=1000*double(steps.size())/total;std::sort(steps.begin(),steps.end());p99=steps[size_t(std::ceil(0.99*double(steps.size())))-1];}
    std::cout<<"probe self-timing (not gate evidence): frames="<<frames<<", mean fps="<<fps<<", p99 ms="<<p99<<'\n';
}
int runGpu(const Assets& a,const Options& opt){
    if(opt.geometry)return geometryCheck(a,opt);
    fs::create_directories(opt.out);require(!fs::exists(opt.out/"run.json")&&!fs::exists(opt.out/"trace.jsonl"),"output directory already contains a run; choose a fresh --out");
    std::string build=buildIdentity();std::vector<uint32_t> dummySample{0};Gpu gpu(a,opt,dummySample);
    bool labelScene=opt.scene=="w3"||opt.scene=="w4";auto labels=a.oracle[0];uint64_t authoritative=0;
    unsigned expected=0,bound=0;bool injectionApplied=false;std::optional<unsigned> pending;
    LabelRecords records;std::vector<Trace> trace;trace.reserve(size_t(opt.duration*200));
    Matrix q=identity();int64_t duration=turnTicks(opt.turnMs,gpu.frequency),start=0,stop=0,renderStart=qpc();uint64_t serial=0;
    bool running=true;MSG message{};
    while(running){
        while(PeekMessageW(&message,nullptr,0,0,PM_REMOVE)){if(message.message==WM_QUIT)running=false;TranslateMessage(&message);DispatchMessageW(&message);}
        if(!running)break;
        gpu.begin(serial);
        // This read is after the previous Present returned and after the in-flight wait,
        // before this frame's upload, draw and Present. It is the single trace clock.
        int64_t now=qpc();if(!start&&double(now-renderStart)/double(gpu.frequency)>=opt.preroll){start=now;gpu.foregroundAtStart=GetForegroundWindow()==gpu.hwnd;}
        bool timed=start!=0;if(timed&&double(now-start)/double(gpu.frequency)>=opt.duration){stop=now;break;}
        uint64_t frameIndex=uint64_t(trace.size());Turn turn{0,0,0};if(timed)turn=turnAt(now,start,duration,a.angle);
        bool copy=false;
        if(labelScene&&timed){
            if(turn.index>authoritative){
                pending.reset();unsigned old=expected;expected=(expected+1)%3;
                while(authoritative<turn.index){a.move(labels,authoritative);++authoritative;}
                auto upload=labels;bool fault=turn.index==20&&!injectionApplied&&!opt.inject.empty();if(fault)injectionApplied=true;
                if(fault)injectLabels(upload,opt.inject);
                gpu.uploadLabels(expected,upload);gpu.labelRevision[expected]=authoritative;
                records.uploads.push_back({frameIndex,authoritative,uint64_t(expected+1)});
                if(fault&&(opt.inject=="delay-adoption"||opt.inject=="stale-binding")){
                    bound=old;pending=expected;copy=opt.inject=="stale-binding";
                }else {bound=expected;copy=true;}
            }else if(pending){bound=*pending;pending.reset();copy=true;}
            records.uses.push_back({frameIndex,turn.index,gpu.labelRevision[bound],uint64_t(bound+1),uint64_t(expected+1)});
        }
        if(opt.scene=="w2"||opt.scene=="w3")rotate(q,0,3,0.002);
        float theta=opt.scene=="w3"&&timed?float(turn.theta):0;
        gpu.draw(constants(a,q,float(gpu.width)/float(gpu.height),theta),bound,gpu.drawPso.Get());
        if(copy){auto slot=uint64_t(records.copies.size());gpu.copyLabels(bound,slot);records.copies.push_back({frameIndex,gpu.labelRevision[bound],uint64_t(bound+1),uint64_t(expected+1),slot});}
        if(timed)trace.push_back({frameIndex,now,labelScene?gpu.labelRevision[bound]:0,opt.scene=="w3",turn.index,turn.phase});
        gpu.sampleMemory();gpu.submit(true);++serial;
    }
    if(!start)start=qpc();if(!stop)stop=qpc();gpu.idle();
    Json check;
    if(labelScene){
        void* mapped=nullptr;D3D12_RANGE range{0,SIZE_T(records.copies.size()*LabelBytes)};hr(gpu.readback.resource->Map(0,&range,&mapped),"map preserved label readbacks after trace");
        check=checkLabels(a,records,[&](const Copy& c){return std::span(static_cast<const uint32_t*>(mapped)+c.slot*Slots,size_t(Slots));});D3D12_RANGE none{0,0};gpu.readback.resource->Unmap(0,&none);
        if(!opt.inject.empty()&&!injectionApplied){check["status"]="fail";check["injection_not_reached"]=true;}
        std::cout<<"label check: "<<check.at("status").string()<<", copies="<<records.copies.size()<<'\n';
    }
    Options written=opt;if(written.runId.empty())written.runId="sb-"+opt.scene+'-'+std::to_string(GetCurrentProcessId())+'-'+std::to_string(start);
    Json run=runJson(written,gpu.frequency,start,stop,build,gpu.environment(),check,gpu.vramPeak);run["frames"]=uint64_t(trace.size());
    run["window"]=Json::Object{{"topmost",true},{"foreground_at_trace_start",gpu.foregroundAtStart},{"presents",gpu.presents},{"presents_occluded",gpu.occluded}};run["injection_applied"]=injectionApplied;
    writeText(opt.out/"run.json",run.dump()+'\n');std::ostringstream lines;for(const auto& entry:trace)lines<<entry.json().dump()<<'\n';writeText(opt.out/"trace.jsonl",lines.str());
    selfTiming(trace,gpu.frequency,start,stop);
    std::cout<<"window: foreground at trace start "<<(gpu.foregroundAtStart?"yes":"no")<<", occluded presents "<<gpu.occluded<<" of "<<gpu.presents<<std::endl;
    return labelScene&&check.at("status").string()=="fail"?2:0;
}
}
