#include "probe.h"
#include <windows.h>
#include <d3d12.h>
#include <dxgi1_6.h>
#include <dwmapi.h>
#include <powersetting.h>
#include <wrl/client.h>
#include <algorithm>
#include <atomic>
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
struct DisplayRequest {
    DisplayRequest(){require(SetThreadExecutionState(ES_CONTINUOUS|ES_SYSTEM_REQUIRED|ES_DISPLAY_REQUIRED)!=0,"cannot keep the display on");}
    ~DisplayRequest(){SetThreadExecutionState(ES_CONTINUOUS);}
};
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
    uint32_t poseEnabled,overlayMode,overlayCount,poseCount;
    float previewAngle,maxAngle,pad0=0,pad1=0;
};
static_assert(sizeof(Constants)==176);
static Constants constants(const Assets& a,const Matrix& q,float aspect,float theta,uint32_t count=0){
    uint32_t mode=a.opt.overlay=="grips"?1:a.opt.overlay=="certificates"?2:a.opt.overlay=="straddles"?3:a.opt.overlay=="angle"?4:0;
    auto points=mode==1?a.grips.size():mode==2||mode==3?a.certificates.size():mode==4?64:0;
    return {q,a.normal,a.radius,0.76f,0.82f,1.18f,1.15f,aspect,theta,count,a.planeU,a.planeV,
            a.opt.scene=="wj"?1u:0u,mode,uint32_t(points),a.maxPoses,std::abs(theta),float(a.angle)};
}
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
    ComPtr<ID3D12PipelineState> drawPso,countPso,computePso,latticePso,overlayPso;
    ComPtr<IDXGISwapChain3> swap;
    ComPtr<ID3D12DescriptorHeap> rtvHeap,dsvHeap;
    std::array<ComPtr<ID3D12Resource>,3> targets;
    ComPtr<ID3D12Resource> depth;
    std::array<Frame,2> frame;
    std::array<Buffer,6> geometry; // vertices, ids, centers, frames, animated flags, samples
    std::array<Buffer,3> labels;
    std::array<std::array<Buffer,3>,3> poseBuffers;
    std::array<uint32_t,3> poseCounts{};
    std::array<Buffer,5> extra; // slot->piece, destination, overlays, straddles, offsets
    uint64_t rowBytes=0,latticeBytes=0,readStride=0;
    unsigned drawBinding=0;
    std::array<uint64_t,3> labelRevision{};
    Buffer readback;
    HANDLE event=nullptr;
    HWND hwnd=nullptr;
    uint64_t nextFence=1,readbackSlots=0;
    UINT width=2560,height=1600,refresh=60,rtvStep=0;
    int64_t frequency=0;
    bool tearing=false;
    double vramPeak=0;
    uint64_t presents=0;
    bool foregroundAtStart=false;
    // Conditions sampled every 100 ms of the trace. A flip-model Present never returns
    // DXGI_STATUS_OCCLUDED, so visibility comes from the window itself, and the power
    // source must hold for the whole trace, not only when run.json is written.
    uint64_t conditionSamples=0,samplesNotVisible=0,samplesCovered=0,samplesNotForeground=0,samplesMains=0,samplesBattery=0;
    std::atomic<int> powerMode{-1}; // EFFECTIVE_POWER_MODE, set by the notification callback
    int powerModeAtStart=-1;bool powerModeChanged=false;void* powerRegistration=nullptr;
    std::string adapterName,driver;
    Frame* active=nullptr;

    Gpu(const Assets& a,const Options& options,const std::vector<uint32_t>& sample):opt(options){
        rowBytes=uint64_t(a.maxPoses)*64;latticeBytes=uint64_t(a.maxPoses)*4;
        readStride=LabelBytes+(opt.scene=="wj"?IndexBytes+rowBytes+latticeBytes:0);
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
            fctx.upload=buffer(LabelBytes+IndexBytes+rowBytes+latticeBytes,D3D12_HEAP_TYPE_UPLOAD,D3D12_RESOURCE_STATE_GENERIC_READ);
            D3D12_RANGE none{0,0};hr(fctx.cb.resource->Map(0,&none,reinterpret_cast<void**>(&fctx.cbData)),"map constants");hr(fctx.upload.resource->Map(0,&none,reinterpret_cast<void**>(&fctx.uploadData)),"map label upload ring");
        }
        hr(device->CreateCommandList(0,D3D12_COMMAND_LIST_TYPE_DIRECT,frame[0].allocator.Get(),nullptr,IID_PPV_ARGS(&list)),"CreateCommandList");hr(list->Close(),"initial Close");
        makeRoot();
        begin(0);
        std::vector<Buffer> uploads;
        auto put=[&](auto& values){return staticBuffer(values.data(),values.size()*sizeof(values[0]),uploads);};
        geometry[0]=put(a.vertices);geometry[1]=put(a.local);geometry[2]=put(a.centers);geometry[3]=put(a.frames);geometry[4]=put(a.flags);geometry[5]=put(sample);
        for(auto& label:labels)label=put(a.oracle[0]);
        extra[0]=put(a.slotPiece);std::vector<uint32_t> destination(Slots);for(uint32_t i=0;i<Slots;++i)destination[i]=i;extra[1]=put(destination);
        std::vector<OverlayPoint> points=opt.overlay=="grips"?a.grips:a.certificates;if(points.empty())points.push_back({});extra[2]=put(points);
        extra[3]=put(a.straddle);extra[4]=put(a.offsets);
        for(unsigned ring=0;ring<3;++ring){
            const auto raw=a.poses[0].raw();const uint64_t sizes[]={IndexBytes,rowBytes,latticeBytes};
            for(size_t j=0;j<3;++j){std::vector<uint8_t> padded(size_t(sizes[j]),0);std::copy(raw[j].begin(),raw[j].end(),padded.begin());poseBuffers[ring][j]=put(padded);}
            poseCounts[ring]=a.poses[0].count();
        }
        submit(false);idle(); // The static upload resources can now be released.
        makeTargets();
        drawPso=graphics("draw_vs.dxil");
        if(opt.overlay!="none")overlayPso=graphics("overlay_vs.dxil","overlay_ps.dxil");
        if((opt.geometry||opt.lattice)){
            countPso=graphics("count_vs.dxil");
            auto compute=[&](const char* name){auto code=bytes(exeDir()/name);D3D12_COMPUTE_PIPELINE_STATE_DESC p{};p.pRootSignature=root.Get();p.CS={code.data(),code.size()};ComPtr<ID3D12PipelineState> pipeline;hr(device->CreateComputePipelineState(&p,IID_PPV_ARGS(&pipeline)),"compute pipeline");return pipeline;};
            computePso=compute("geometry_cs.dxil");latticePso=compute("lattice_cs.dxil");
        }
        else if(opt.scene=="w3"||opt.scene=="wj"){
            double slots=std::ceil(opt.duration*1000/opt.turnMs)+2;
            require(slots<double(std::numeric_limits<uint64_t>::max()/readStride),"revision readback size overflow");readbackSlots=uint64_t(slots);
            try{readback=buffer(readbackSlots*readStride,D3D12_HEAP_TYPE_READBACK,D3D12_RESOURCE_STATE_COPY_DEST);}
            catch(const std::exception&){throw std::runtime_error("cannot allocate preserved revision readbacks ("+std::to_string(readbackSlots*readStride/1048576)+" MiB); no timing run started");}
        }
        // Last, after every step that can throw: only a constructed Gpu runs ~Gpu, which
        // unregisters (waiting for running callbacks) before the members are destroyed.
        // The callback reports the current effective power mode at registration and every change.
        if(FAILED(PowerRegisterForEffectivePowerModeNotifications(EFFECTIVE_POWER_MODE_V2,&Gpu::onPowerMode,this,&powerRegistration)))powerRegistration=nullptr;
    }
    ~Gpu(){
        if(powerRegistration)PowerUnregisterFromEffectivePowerModeNotifications(powerRegistration);
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
        if(present){hr(swap->Present(opt.vsync?1:0,!opt.vsync&&tearing?DXGI_PRESENT_ALLOW_TEARING:0),"Present");++presents;}
        active->fence=nextFence++;hr(queue->Signal(fence.Get(),active->fence),"frame Signal");
    }
    void makeRoot(){
        D3D12_ROOT_PARAMETER params[18]{};params[0].ParameterType=D3D12_ROOT_PARAMETER_TYPE_CBV;params[0].Descriptor.ShaderRegister=0;
        for(UINT i=1;i<=7;++i){params[i].ParameterType=D3D12_ROOT_PARAMETER_TYPE_SRV;params[i].Descriptor.ShaderRegister=i-1;}
        for(UINT i=8;i<=9;++i){params[i].ParameterType=D3D12_ROOT_PARAMETER_TYPE_UAV;params[i].Descriptor.ShaderRegister=i-8;}
        for(UINT i=10;i<18;++i){params[i].ParameterType=D3D12_ROOT_PARAMETER_TYPE_SRV;params[i].Descriptor.ShaderRegister=i-3;}
        D3D12_ROOT_SIGNATURE_DESC d{};d.NumParameters=18;d.pParameters=params;d.Flags=D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT;
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
                if(!(opt.geometry||opt.lattice)){
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
        D3D12_DESCRIPTOR_HEAP_DESC rh{};rh.Type=D3D12_DESCRIPTOR_HEAP_TYPE_RTV;rh.NumDescriptors=3;hr(device->CreateDescriptorHeap(&rh,IID_PPV_ARGS(&rtvHeap)),"RTV heap");rtvStep=device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
        for(UINT i=0;i<((opt.geometry||opt.lattice)?1u:3u);++i)device->CreateRenderTargetView(targets[i].Get(),nullptr,rtv(i));
        D3D12_DESCRIPTOR_HEAP_DESC dh{};dh.Type=D3D12_DESCRIPTOR_HEAP_TYPE_DSV;dh.NumDescriptors=1;hr(device->CreateDescriptorHeap(&dh,IID_PPV_ARGS(&dsvHeap)),"DSV heap");D3D12_CLEAR_VALUE dc{};dc.Format=DXGI_FORMAT_D32_FLOAT;dc.DepthStencil.Depth=1;
        depth=texture(DXGI_FORMAT_D32_FLOAT,D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL,D3D12_RESOURCE_STATE_DEPTH_WRITE,1,&dc);
        device->CreateDepthStencilView(depth.Get(),nullptr,dsvHeap->GetCPUDescriptorHandleForHeapStart());

    }
    D3D12_CPU_DESCRIPTOR_HANDLE rtv(UINT i){auto h=rtvHeap->GetCPUDescriptorHandleForHeapStart();h.ptr+=SIZE_T(i)*rtvStep;return h;}
    ComPtr<ID3D12PipelineState> graphics(const char* shader,const char* pixel="draw_ps.dxil"){
        auto vs=bytes(exeDir()/shader),ps=bytes(exeDir()/pixel);D3D12_GRAPHICS_PIPELINE_STATE_DESC d{};d.pRootSignature=root.Get();d.VS={vs.data(),vs.size()};d.PS={ps.data(),ps.size()};
        auto& blend=d.BlendState.RenderTarget[0];blend.SrcBlend=D3D12_BLEND_ONE;blend.DestBlend=D3D12_BLEND_ZERO;blend.BlendOp=D3D12_BLEND_OP_ADD;blend.SrcBlendAlpha=D3D12_BLEND_ONE;blend.DestBlendAlpha=D3D12_BLEND_ZERO;blend.BlendOpAlpha=D3D12_BLEND_OP_ADD;blend.LogicOp=D3D12_LOGIC_OP_NOOP;blend.RenderTargetWriteMask=D3D12_COLOR_WRITE_ENABLE_ALL;
        d.SampleMask=UINT_MAX;d.RasterizerState.FillMode=D3D12_FILL_MODE_SOLID;d.RasterizerState.CullMode=D3D12_CULL_MODE_NONE;d.RasterizerState.DepthClipEnable=TRUE;
        d.DepthStencilState.DepthEnable=TRUE;d.DepthStencilState.DepthWriteMask=D3D12_DEPTH_WRITE_MASK_ALL;d.DepthStencilState.DepthFunc=D3D12_COMPARISON_FUNC_LESS_EQUAL;
        if(std::string(shader)=="overlay_vs.dxil"){d.DepthStencilState.DepthEnable=FALSE;d.DepthStencilState.DepthWriteMask=D3D12_DEPTH_WRITE_MASK_ZERO;}
        d.DepthStencilState.StencilReadMask=D3D12_DEFAULT_STENCIL_READ_MASK;d.DepthStencilState.StencilWriteMask=D3D12_DEFAULT_STENCIL_WRITE_MASK;
        D3D12_DEPTH_STENCILOP_DESC stencil{};stencil.StencilFailOp=stencil.StencilDepthFailOp=stencil.StencilPassOp=D3D12_STENCIL_OP_KEEP;stencil.StencilFunc=D3D12_COMPARISON_FUNC_ALWAYS;d.DepthStencilState.FrontFace=d.DepthStencilState.BackFace=stencil;
        d.PrimitiveTopologyType=D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;d.NumRenderTargets=1;d.RTVFormats[0]=DXGI_FORMAT_R8G8B8A8_UNORM;d.DSVFormat=DXGI_FORMAT_D32_FLOAT;d.SampleDesc.Count=1;
        ComPtr<ID3D12PipelineState> p;hr(device->CreateGraphicsPipelineState(&d,IID_PPV_ARGS(&p)),"graphics pipeline");return p;
    }
    void bind(const Constants& c,unsigned labelIndex,bool compute){
        std::memcpy(active->cbData,&c,sizeof(c));
        if(compute)list->SetComputeRootSignature(root.Get());else list->SetGraphicsRootSignature(root.Get());
        auto cb=active->cb.resource->GetGPUVirtualAddress();if(compute)list->SetComputeRootConstantBufferView(0,cb);else list->SetGraphicsRootConstantBufferView(0,cb);
        for(UINT i=0;i<7;++i){auto address=(i<5?geometry[i]:i==5?labels[labelIndex]:geometry[5]).resource->GetGPUVirtualAddress();if(compute)list->SetComputeRootShaderResourceView(i+1,address);else list->SetGraphicsRootShaderResourceView(i+1,address);}
        const Buffer* additional[]={&extra[0],&poseBuffers[labelIndex][0],&poseBuffers[labelIndex][1],&poseBuffers[labelIndex][2],&extra[1],&extra[2],&extra[3],&extra[4]};
        for(UINT i=0;i<8;++i){auto address=additional[i]->resource->GetGPUVirtualAddress();if(compute)list->SetComputeRootShaderResourceView(i+10,address);else list->SetGraphicsRootShaderResourceView(i+10,address);}
        if(!compute)drawBinding=labelIndex;
    }
    void uploadLabels(unsigned index,const std::vector<uint32_t>& values){
        std::memcpy(active->uploadData,values.data(),size_t(LabelBytes));transition(labels[index],D3D12_RESOURCE_STATE_COPY_DEST);list->CopyBufferRegion(labels[index].resource.Get(),0,active->upload.resource.Get(),0,LabelBytes);transition(labels[index],D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
    }
    void uploadPose(unsigned index,const PoseState& state){
        auto raw=state.raw();const uint64_t offsets[]={LabelBytes,LabelBytes+IndexBytes,LabelBytes+IndexBytes+rowBytes};
        require(state.count()*uint64_t(64)<=rowBytes,"pose table exceeds allocated capacity");
        for(size_t j=0;j<3;++j){auto& target=poseBuffers[index][j];std::memset(active->uploadData+offsets[j],0,size_t(target.size));std::memcpy(active->uploadData+offsets[j],raw[j].data(),raw[j].size());
            transition(target,D3D12_RESOURCE_STATE_COPY_DEST);list->CopyBufferRegion(target.resource.Get(),0,active->upload.resource.Get(),offsets[j],target.size);transition(target,D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);}
        poseCounts[index]=state.count();
    }
    void draw(const Constants& c,unsigned labelIndex,ID3D12PipelineState* pipeline,Buffer* counter=nullptr){
        UINT back=(opt.geometry||opt.lattice)?0:swap->GetCurrentBackBufferIndex();
        if(!(opt.geometry||opt.lattice))transition(targets[back].Get(),D3D12_RESOURCE_STATE_PRESENT,D3D12_RESOURCE_STATE_RENDER_TARGET);
        list->SetPipelineState(pipeline);bind(c,labelIndex,false);
        if(counter)list->SetGraphicsRootUnorderedAccessView(8,counter->resource->GetGPUVirtualAddress());
        D3D12_VIEWPORT viewport{0,0,float(width),float(height),0,1};D3D12_RECT rect{0,0,LONG(width),LONG(height)};list->RSSetViewports(1,&viewport);list->RSSetScissorRects(1,&rect);
        auto target=rtv(back);auto dsv=dsvHeap->GetCPUDescriptorHandleForHeapStart();list->OMSetRenderTargets(1,&target,FALSE,&dsv);
        const float colour[4]={0.13f,0.145f,0.16f,1};list->ClearRenderTargetView(target,colour,0,nullptr);list->ClearDepthStencilView(dsv,D3D12_CLEAR_FLAG_DEPTH,1,0,0,nullptr);list->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        list->DrawInstanced(Vertices,Cells,0,0);
        if(overlayPso){list->SetPipelineState(overlayPso.Get());list->DrawInstanced(6,c.overlayCount,0,0);}
        if(!(opt.geometry||opt.lattice))transition(targets[back].Get(),D3D12_RESOURCE_STATE_RENDER_TARGET,D3D12_RESOURCE_STATE_PRESENT);
    }
    void copyBound(uint64_t slot){
        require(slot<readbackSlots,"preserved revision readback capacity exceeded");
        auto copy=[&](Buffer& bound,uint64_t offset,uint64_t size){transition(bound,D3D12_RESOURCE_STATE_COPY_SOURCE);
            list->CopyBufferRegion(readback.resource.Get(),slot*readStride+offset,bound.resource.Get(),0,size);transition(bound,D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);};
        copy(labels[drawBinding],0,LabelBytes);
        if(opt.scene=="wj"){
            // Exactly the three root SRVs bound by draw(), after all draws in this list.
            copy(poseBuffers[drawBinding][0],LabelBytes,IndexBytes);
            copy(poseBuffers[drawBinding][1],LabelBytes+IndexBytes,uint64_t(poseCounts[drawBinding])*64);
            copy(poseBuffers[drawBinding][2],LabelBytes+IndexBytes+rowBytes,uint64_t(poseCounts[drawBinding])*4);
        }
    }
    void sampleMemory(){DXGI_QUERY_VIDEO_MEMORY_INFO info{};hr(memoryAdapter->QueryVideoMemoryInfo(0,DXGI_MEMORY_SEGMENT_GROUP_LOCAL,&info),"QueryVideoMemoryInfo");vramPeak=std::max(vramPeak,double(info.CurrentUsage)/1048576);}
    static VOID WINAPI onPowerMode(EFFECTIVE_POWER_MODE mode,VOID* context){static_cast<Gpu*>(context)->powerMode.store(int(mode));}
    static const char* powerModeName(int mode){
        switch(mode){
        case EffectivePowerModeBatterySaver:return "battery_saver";case EffectivePowerModeBetterBattery:return "better_battery";
        case EffectivePowerModeBalanced:return "balanced";case EffectivePowerModeHighPerformance:return "high_performance";
        case EffectivePowerModeMaxPerformance:return "max_performance";case EffectivePowerModeGameMode:return "game_mode";
        case EffectivePowerModeMixedReality:return "mixed_reality";default:return "unknown";
        }
    }
    bool covered(){
        // IsWindowVisible is true for a window that another window hides completely, so hit-test
        // the centre and four inner points: each must belong to this window.
        RECT r{};if(!GetWindowRect(hwnd,&r))return true;LONG w=r.right-r.left,h=r.bottom-r.top;
        const POINT points[]={{r.left+w/2,r.top+h/2},{r.left+w/10,r.top+h/10},{r.right-w/10,r.top+h/10},{r.left+w/10,r.bottom-h/10},{r.right-w/10,r.bottom-h/10}};
        for(const auto& point:points){HWND hit=WindowFromPoint(point);if(!hit||GetAncestor(hit,GA_ROOT)!=hwnd)return true;}
        return false;
    }
    void sampleConditions(){
        BOOL cloaked=FALSE;bool hidden=SUCCEEDED(DwmGetWindowAttribute(hwnd,DWMWA_CLOAKED,&cloaked,sizeof(cloaked)))&&cloaked;
        bool coveredNow=covered();if(coveredNow)++samplesCovered;
        if(conditionSamples==0)powerModeAtStart=powerMode.load();else if(powerMode.load()!=powerModeAtStart)powerModeChanged=true;
        ++conditionSamples;if(!IsWindowVisible(hwnd)||IsIconic(hwnd)||hidden||coveredNow)++samplesNotVisible;if(GetForegroundWindow()!=hwnd)++samplesNotForeground;
        SYSTEM_POWER_STATUS power{};if(GetSystemPowerStatus(&power)!=0){if(power.ACLineStatus==1)++samplesMains;else if(power.ACLineStatus==0)++samplesBattery;}
    }
    bool outputOnAdapter(){
        // True when the window's monitor is an output of the rendering adapter (a discrete-only
        // or MUX mode); false when another adapter scans it out (hybrid copy to the iGPU).
        HMONITOR monitor=MonitorFromWindow(hwnd,MONITOR_DEFAULTTOPRIMARY);
        for(UINT i=0;;++i){ComPtr<IDXGIOutput> output;if(adapter->EnumOutputs(i,&output)==DXGI_ERROR_NOT_FOUND)return false;DXGI_OUTPUT_DESC d{};if(SUCCEEDED(output->GetDesc(&d))&&d.Monitor==monitor)return true;}
    }
    Json environment(){
        if(conditionSamples==0)sampleConditions(); // a run stopped before its trace started
        std::string source=samplesMains==conditionSamples?"mains":samplesBattery==conditionSamples?"battery":samplesMains+samplesBattery==conditionSamples?"changed":"unknown";
        std::string mode=powerModeChanged?"changed":powerModeName(powerModeAtStart);
        std::string presenting=adapterName+", driver "+driver+(outputOnAdapter()?", drives the window's display":", display driven by another adapter");
        MONITORINFOEXW info{};info.cbSize=sizeof(info);require(GetMonitorInfoW(MonitorFromWindow(hwnd,MONITOR_DEFAULTTOPRIMARY),&info)!=0,"window monitor unavailable");DEVMODEW display{};display.dmSize=sizeof(display);require(EnumDisplaySettingsW(info.szDevice,ENUM_CURRENT_SETTINGS,&display)!=0,"window monitor mode unavailable");
        // Keys in the shape the gate summary publishes (tools/perf/b412_summary.py public_environment).
        return Json::Object{{"power_source",source},{"power_mode",mode},{"presenting_adapter",presenting},{"presentation_interval",opt.vsync?1:0},{"declared",opt.declared},
            {"display",Json::Object{{"width",display.dmPelsWidth},{"height",display.dmPelsHeight},{"refresh_hz",display.dmDisplayFrequency}}},{"backbuffer",Json::Object{{"width",width},{"height",height}}},
            {"power_samples",Json::Object{{"samples",conditionSamples},{"mains",samplesMains},{"battery",samplesBattery}}},{"vsync",opt.vsync},{"tearing",tearing},{"adapter",adapterName},{"driver",driver},{"msaa",1},{"warp",opt.warp}};
    }
};

static int geometryCheck(const Assets& a,const Options& opt){
    auto sample=a.samples();auto reference=repoRoot()/"work/experiments/renderer-wj";
    bool jumbling=opt.scene=="wj";
    if(!jumbling)reference=repoRoot()/"work/experiments/renderer-sb/reference";
    auto prefix=jumbling?"ref_"+opt.menu+"_":std::string();auto index=readJson(reference/(prefix+"index.json"));
    for(const auto& [name,digest]:index.at("files").object())require(sha256(bytes(reference/name))==digest.string(),"immutable geometry reference hash mismatch");
    auto supplied=bytes(reference/(prefix+"sample.u32"));
    require(supplied.size()==sample.size()*4&&std::memcmp(supplied.data(),sample.data(),supplied.size())==0,"geometry sample differs from reference rule");
    Gpu gpu(a,opt,sample);
    auto result=gpu.buffer(sample.size()*12,D3D12_HEAP_TYPE_DEFAULT,D3D12_RESOURCE_STATE_UNORDERED_ACCESS,D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
    auto read=gpu.buffer(result.size,D3D12_HEAP_TYPE_READBACK,D3D12_RESOURCE_STATE_COPY_DEST);
    auto counter=gpu.buffer(Cells*4,D3D12_HEAP_TYPE_DEFAULT,D3D12_RESOURCE_STATE_COPY_DEST,D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
    auto counterRead=gpu.buffer(counter.size,D3D12_HEAP_TYPE_READBACK,D3D12_RESOURCE_STATE_COPY_DEST);
    auto zero=gpu.buffer(counter.size,D3D12_HEAP_TYPE_UPLOAD,D3D12_RESOURCE_STATE_GENERIC_READ);
    void* mapped=nullptr;D3D12_RANGE none{0,0};hr(zero.resource->Map(0,&none,&mapped),"map counter zero");std::memset(mapped,0,size_t(counter.size));zero.resource->Unmap(0,nullptr);
    gpu.begin(0);gpu.list->CopyBufferRegion(counter.resource.Get(),0,zero.resource.Get(),0,counter.size);gpu.transition(counter,D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
    gpu.draw(constants(a,identity(),1.6f,0),0,gpu.countPso.Get(),&counter);gpu.transition(counter,D3D12_RESOURCE_STATE_COPY_SOURCE);gpu.list->CopyBufferRegion(counterRead.resource.Get(),0,counter.resource.Get(),0,counter.size);gpu.submit(false);gpu.idle();
    D3D12_RANGE range{0,SIZE_T(counter.size)};hr(counterRead.resource->Map(0,&range,&mapped),"map invocation counts");uint64_t countFailures=0;
    const auto* counts=static_cast<const uint32_t*>(mapped);for(uint32_t cell=0;cell<Cells;++cell)if(counts[cell]!=Vertices)++countFailures;counterRead.resource->Unmap(0,&none);
    Json::Array outputs,checks;
    if(jumbling)outputs=index.at("outputs").array();
    else for(const auto& camera:a.cameras.at("cameras").array())for(const auto& [state,theta]:std::array<std::pair<const char*,double>,3>{{{"start",0},{"mid",a.angle*.5},{"end",a.angle}}})
        outputs.emplace_back(Json::Object{{"file",camera.at("name").string()+"_"+state+".f32"},{"camera",camera.at("name")},{"state",state},{"theta",theta}});
    bool failed=countFailures!=0;
    for(const auto& output:outputs){
        auto q=identity();bool found=false;
        for(const auto& camera:a.cameras.at("cameras").array())if(camera.at("name").string()==output.at("camera").string()){
            for(const auto& rotation:camera.at("rotations").array())rotate(q,int(rotation.array()[0].integer()),int(rotation.array()[1].integer()),rotation.array()[2].number());found=true;}
        require(found,"reference names unknown camera");
        gpu.begin(0);if(jumbling)gpu.uploadPose(0,a.poseState(output.at("stage").string()));
        gpu.list->SetPipelineState(gpu.computePso.Get());gpu.bind(constants(a,q,1.6f,float(output.at("theta").number()),uint32_t(sample.size())),0,true);
        gpu.list->SetComputeRootUnorderedAccessView(9,result.resource->GetGPUVirtualAddress());gpu.list->Dispatch(UINT((sample.size()+63)/64),1,1);
        gpu.transition(result,D3D12_RESOURCE_STATE_COPY_SOURCE);gpu.list->CopyBufferRegion(read.resource.Get(),0,result.resource.Get(),0,result.size);gpu.transition(result,D3D12_RESOURCE_STATE_UNORDERED_ACCESS);gpu.submit(false);gpu.idle();
        auto referenceRaw=bytes(reference/output.at("file").string());require(referenceRaw.size()==result.size,"geometry reference length mismatch");
        std::vector<float> expected(referenceRaw.size()/4);std::memcpy(expected.data(),referenceRaw.data(),referenceRaw.size());
        range={0,SIZE_T(result.size)};hr(read.resource->Map(0,&range,&mapped),"map GPU projections");auto values=static_cast<const float*>(mapped);double maxError=0;uint64_t errors=0;
        for(size_t i=0;i<expected.size();++i){double error=std::abs(double(values[i])-expected[i]);if(!std::isfinite(values[i])||error>1e-4+1e-4*std::abs(double(expected[i])))++errors;maxError=std::max(maxError,error);}
        read.resource->Unmap(0,&none);failed|=errors!=0;
        checks.emplace_back(Json::Object{{"file",output.at("file")},{"samples",uint64_t(sample.size())},{"max_abs_error",maxError},{"failures",errors},{"status",errors?"fail":"pass"}});
    }
    Json check=Json::Object{{"format","magic600-wj-geometry-check/1"},{"status",failed?"fail":"pass"},{"build_identity",buildIdentity()},{"results",checks},
                            {"cells",Cells},{"vertices_per_cell",Vertices},{"invocation_count_failures",countFailures}};
    fs::create_directories(opt.out);writeText(opt.out/"geometry_check.json",check.dump()+'\n');std::cout<<"GPU geometry check: "<<check.at("status").string()<<'\n';return failed?1:0;
}
static int latticeCheck(const Assets& a,const Options& opt){
    Gpu gpu(a,opt,{0});auto result=gpu.buffer(uint64_t(Slots)*16,D3D12_HEAP_TYPE_DEFAULT,D3D12_RESOURCE_STATE_UNORDERED_ACCESS,D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
    auto read=gpu.buffer(result.size,D3D12_HEAP_TYPE_READBACK,D3D12_RESOURCE_STATE_COPY_DEST);Json::Array states;bool failed=false;
    for(const char* stage:{"lattice-start","lattice-retained"}){
        auto directory=opt.data/stage;auto header=readJson(directory/"header.json");
        auto labelRaw=bytes(directory/"labels.u32"),destRaw=bytes(directory/"destination.u32");
        require(labelRaw.size()==LabelBytes&&destRaw.size()==LabelBytes,"lattice control length mismatch");
        require(sha256(labelRaw)==header.at("files").at("labels.u32").at("sha256").string()&&sha256(destRaw)==header.at("files").at("destination.u32").at("sha256").string(),"lattice control hash mismatch");
        std::vector<uint32_t> labels(Slots);std::memcpy(labels.data(),labelRaw.data(),labelRaw.size());
        gpu.begin(0);gpu.uploadLabels(0,labels);gpu.uploadPose(0,a.poseState(stage));
        std::vector<Buffer> temporary;gpu.extra[1]=gpu.staticBuffer(destRaw.data(),destRaw.size(),temporary);
        gpu.bind(constants(a,identity(),1.6f,0),0,true);gpu.list->SetPipelineState(gpu.latticePso.Get());gpu.list->SetComputeRootUnorderedAccessView(9,result.resource->GetGPUVirtualAddress());gpu.list->Dispatch((Slots+63)/64,1,1);
        gpu.transition(result,D3D12_RESOURCE_STATE_COPY_SOURCE);gpu.list->CopyBufferRegion(read.resource.Get(),0,result.resource.Get(),0,result.size);gpu.transition(result,D3D12_RESOURCE_STATE_UNORDERED_ACCESS);gpu.submit(false);gpu.idle();
        void* mapped=nullptr;D3D12_RANGE range{0,SIZE_T(result.size)};hr(read.resource->Map(0,&range,&mapped),"map lattice results");auto values=static_cast<const uint32_t*>(mapped);std::array<uint64_t,4> errors{};
        for(uint32_t slot=0;slot<Slots;++slot)for(size_t j=0;j<4;++j)errors[j]+=values[size_t(slot)*4+j]!=0;
        D3D12_RANGE none{0,0};read.resource->Unmap(0,&none);bool bad=std::any_of(errors.begin(),errors.end(),[](uint64_t e){return e!=0;});failed|=bad;
        states.emplace_back(Json::Object{{"state",stage},{"status",bad?"fail":"pass"},{"slots",Slots},{"label_failures",errors[0]},{"center_failures",errors[1]},{"geometry_bounds_failures",errors[2]},{"lattice_failures",errors[3]}});
    }
    fs::create_directories(opt.out);writeText(opt.out/"lattice_check.json",Json(Json::Object{{"format","magic600-wj-lattice-check/1"},{"status",failed?"fail":"pass"},{"build_identity",buildIdentity()},{"states",states}}).dump()+'\n');
    std::cout<<"GPU lattice check: "<<(failed?"fail":"pass")<<'\n';return failed?1:0;
}
static void selfTiming(const std::vector<Trace>& trace,int64_t frequency,int64_t start,int64_t stop){
    int64_t begin=start,end=stop;if(stop-start>=190*frequency){begin=start+10*frequency;end=start+190*frequency;}
    std::vector<double> steps;uint64_t frames=0;
    for(size_t i=0;i<trace.size();++i)if(trace[i].qpc>=begin&&trace[i].qpc<end){++frames;if(i>0)steps.push_back(double(trace[i].qpc-trace[i-1].qpc)*1000/double(frequency));}
    double fps=0,p99=0;if(!steps.empty()){double total=0;for(double step:steps)total+=step;fps=1000*double(steps.size())/total;std::sort(steps.begin(),steps.end());p99=steps[size_t(std::ceil(0.99*double(steps.size())))-1];}
    std::cout<<"probe self-timing (not gate evidence): frames="<<frames<<", mean fps="<<fps<<", p99 ms="<<p99<<'\n';
}
int runGpu(const Assets& a,const Options& opt){
    if(opt.geometry)return geometryCheck(a,opt);if(opt.lattice)return latticeCheck(a,opt);
    DisplayRequest displayRequest;
    fs::create_directories(opt.out);require(!fs::exists(opt.out/"run.json")&&!fs::exists(opt.out/"trace.jsonl"),"choose a fresh run directory");
    std::string build=buildIdentity();Gpu gpu(a,opt,{0});bool jumbling=opt.scene=="wj";
    auto labels=a.oracle[0];uint64_t authoritative=0;unsigned expected=0,bound=0;bool injectionApplied=false,initial=true;
    std::optional<unsigned> pending;LabelRecords records;std::vector<Trace> trace;trace.reserve(size_t(opt.duration*200));
    Matrix q=identity();int64_t duration=turnTicks(opt.turnMs,gpu.frequency),start=0,stop=0,renderStart=qpc(),nextSample=0;uint64_t serial=0,camera=0;
    bool running=true;MSG message{};
    while(running){
        while(PeekMessageW(&message,nullptr,0,0,PM_REMOVE)){if(message.message==WM_QUIT)running=false;TranslateMessage(&message);DispatchMessageW(&message);}
        if(!running)break;gpu.begin(serial);int64_t now=qpc();
        if(!start&&double(now-renderStart)/double(gpu.frequency)>=opt.preroll){start=now;gpu.foregroundAtStart=GetForegroundWindow()==gpu.hwnd;}
        bool timed=start!=0;if(timed&&double(now-start)/double(gpu.frequency)>=opt.duration){stop=now;break;}
        if(timed&&now>=nextSample){gpu.sampleConditions();nextSample=now+gpu.frequency/10;}
        uint64_t frameIndex=uint64_t(trace.size());Turn turn{0,0,0};if(timed)turn=turnAt(now,start,duration,a.angle);bool copy=false;
        if(timed){
            if(initial||turn.index>authoritative){
                pending.reset();unsigned old=expected;if(!initial)expected=(expected+1)%3;
                if(!jumbling)while(authoritative<turn.index){a.move(labels,authoritative);++authoritative;}else authoritative=turn.index;
                bool fault=turn.index==20&&!injectionApplied&&!opt.inject.empty();if(fault)injectionApplied=true;
                gpu.uploadLabels(expected,labels);
                if(jumbling){auto upload=a.poses[authoritative%2];if(fault)injectPose(upload,a.poses[(authoritative+1)%2],a,opt.inject);gpu.uploadPose(expected,upload);}
                gpu.labelRevision[expected]=authoritative;records.uploads.push_back({frameIndex,authoritative,uint64_t(expected+1)});initial=false;
                if(fault&&(opt.inject=="delay-adoption"||opt.inject=="stale-binding")){bound=old;pending=expected;copy=opt.inject=="stale-binding";}
                else{bound=expected;copy=true;}
            }else if(pending){bound=*pending;pending.reset();copy=true;}
            records.uses.push_back({frameIndex,turn.index,gpu.labelRevision[bound],uint64_t(bound+1),uint64_t(expected+1)});
        }
        rotate(q,0,3,.002);++camera;float theta=timed?float(turn.theta):0;
        gpu.draw(constants(a,q,float(gpu.width)/float(gpu.height),theta),bound,gpu.drawPso.Get());
        if(copy){auto slot=uint64_t(records.copies.size());gpu.copyBound(slot);records.copies.push_back({frameIndex,gpu.labelRevision[gpu.drawBinding],uint64_t(gpu.drawBinding+1),uint64_t(expected+1),slot,gpu.poseCounts[gpu.drawBinding]});}
        if(timed)trace.push_back({frameIndex,now,gpu.labelRevision[bound],true,turn.index,turn.phase,camera});
        gpu.sampleMemory();gpu.submit(true);++serial;
    }
    if(!start)start=qpc();if(!stop)stop=qpc();gpu.idle();
    void* mapped=nullptr;D3D12_RANGE range{0,SIZE_T(gpu.readback.size)};hr(gpu.readback.resource->Map(0,&range,&mapped),"map preserved revision readbacks after capture");auto data=static_cast<const uint8_t*>(mapped);
    auto labelCheck=checkLabels(a,records,[&](const Copy& c){return std::span(reinterpret_cast<const uint32_t*>(data+c.slot*gpu.readStride),size_t(Slots));});
    Json poseCheck;
    if(jumbling)poseCheck=checkPoses(a,records,[&](const Copy& c)->PoseBytes{
        const auto* base=data+c.slot*gpu.readStride+LabelBytes;
        return {std::span(base,size_t(IndexBytes)),std::span(base+IndexBytes,size_t(c.poses)*64),std::span(base+IndexBytes+gpu.rowBytes,size_t(c.poses)*4)};});
    D3D12_RANGE none{0,0};gpu.readback.resource->Unmap(0,&none);
    if(!opt.inject.empty()&&!injectionApplied){labelCheck["status"]="fail";if(jumbling){poseCheck["status"]="fail";poseCheck["injection_not_reached"]=true;}}
    Options written=opt;if(written.runId.empty())written.runId="wj-"+opt.scene+'-'+std::to_string(GetCurrentProcessId())+'-'+std::to_string(start);
    Json run=runJson(written,gpu.frequency,start,stop,build,gpu.environment(),labelCheck,gpu.vramPeak);run["frames"]=uint64_t(trace.size());
    if(jumbling){run["pose_check"]=Json::Object{{"status",poseCheck.at("status")},{"revisions",poseCheck.at("revisions")}};writeText(opt.out/"pose_check.json",poseCheck.dump()+'\n');}
    bool visible=gpu.samplesNotVisible==0,foreground=gpu.samplesNotForeground==0;
    run["window"]=Json::Object{{"topmost",true},{"display_required",true},{"foreground_at_trace_start",gpu.foregroundAtStart},{"sample_period_ms",100},{"samples",gpu.conditionSamples},
        {"samples_not_visible",gpu.samplesNotVisible},{"samples_covered",gpu.samplesCovered},{"samples_not_foreground",gpu.samplesNotForeground},{"visible_throughout",visible},{"foreground_throughout",foreground},{"presents",gpu.presents}};
    run["injection_applied"]=injectionApplied;run["pose_upload"]=jumbling?"whole index, matrix table and lattice table; atomic fenced bundle":"disabled";
    writeText(opt.out/"run.json",run.dump()+'\n');std::ostringstream lines;for(const auto& entry:trace)lines<<entry.json().dump()<<'\n';writeText(opt.out/"trace.jsonl",lines.str());
    selfTiming(trace,gpu.frequency,start,stop);
    std::cout<<"label check: "<<labelCheck.at("status").string()<<"; pose check: "<<(jumbling?poseCheck.at("status").string():"not applicable")<<'\n';
    if(labelCheck.at("status").string()=="fail"||(jumbling&&poseCheck.at("status").string()=="fail"))return 2;
    return visible&&foreground?0:3;
}
}
