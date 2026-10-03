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
#include <wincodec.h>

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
static void png(const fs::path& path,UINT width,UINT height,std::span<const uint8_t> rgba){
    hr(CoInitializeEx(nullptr,COINIT_MULTITHREADED),"snapshot COM initialization");
    struct Uninitialize { ~Uninitialize(){CoUninitialize();} } uninitialize;
    ComPtr<IWICImagingFactory> factory;hr(CoCreateInstance(CLSID_WICImagingFactory,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&factory)),"WIC factory");
    ComPtr<IWICStream> stream;hr(factory->CreateStream(&stream),"WIC stream");hr(stream->InitializeFromFilename(path.c_str(),GENERIC_WRITE),"snapshot file");
    ComPtr<IWICBitmapEncoder> encoder;hr(factory->CreateEncoder(GUID_ContainerFormatPng,nullptr,&encoder),"PNG encoder");hr(encoder->Initialize(stream.Get(),WICBitmapEncoderNoCache),"PNG initialization");
    ComPtr<IWICBitmapFrameEncode> frame;hr(encoder->CreateNewFrame(&frame,nullptr),"PNG frame");hr(frame->Initialize(nullptr),"PNG frame initialization");hr(frame->SetSize(width,height),"PNG size");
    // The PNG encoder takes 32bppBGRA but not 32bppRGBA, so swap red and blue in a copy.
    std::vector<uint8_t> bgra(rgba.begin(),rgba.end());for(size_t i=0;i+3<bgra.size();i+=4)std::swap(bgra[i],bgra[i+2]);
    WICPixelFormatGUID format=GUID_WICPixelFormat32bppBGRA;hr(frame->SetPixelFormat(&format),"PNG format");require(format==GUID_WICPixelFormat32bppBGRA,"WIC did not accept BGRA");
    require(bgra.size()<=UINT_MAX,"snapshot exceeds WIC size limit");hr(frame->WritePixels(height,width*4,UINT(bgra.size()),bgra.data()),"PNG pixels");hr(frame->Commit(),"PNG frame commit");hr(encoder->Commit(),"PNG commit");
}
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
    ComPtr<ID3D12RootSignature> featureRoot;
    ComPtr<ID3D12PipelineState> featurePso;
    Buffer edgeData;
    Buffer sortKeys,sortPrefix,sortBlocks,sortIndices,stickerOffsets;
    std::array<ComPtr<ID3D12PipelineState>,5> sortPso;
    Json sortResult;
    ComPtr<ID3D12RootSignature> postRoot;
    ComPtr<ID3D12PipelineState> postPso;
    ComPtr<ID3D12PipelineState> postBlurPso,postCompositePso;
    ComPtr<ID3D12DescriptorHeap> postHeap;
    ComPtr<ID3D12Resource> postColour;
    std::array<ComPtr<ID3D12Resource>,2> postAux;
    UINT postStep=0;
    ComPtr<IDXGISwapChain3> swap;
    ComPtr<ID3D12DescriptorHeap> rtvHeap,dsvHeap;
    std::array<ComPtr<ID3D12Resource>,3> targets;
    ComPtr<ID3D12Resource> depth,msaaTarget;
    std::array<Frame,2> frame;
    std::array<Buffer,6> geometry; // vertices, ids, centers, frames, animated flags, samples
    std::array<Buffer,3> labels;
    std::array<uint64_t,3> labelRevision{};
    Buffer readback;
    std::vector<Buffer> extraReadbacks; // W3f has no clock-derived upper bound on turn count.
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
        if(opt.feature=="fog"){
            // Separate signature and PSO; neither exists in a baseline run.
            makeFeatureRoot();featurePso=graphics("draw_vs.dxil","fog_ps.dxil",featureRoot.Get());
        }
        if(opt.feature=="outlines"){
            auto edges=edgeMasks(a);begin(0);std::vector<Buffer> upload;edgeData=staticBuffer(edges.triangles.data(),edges.triangles.size()*4,upload);submit(false);idle();
            makeFeatureRoot();featurePso=graphics("outline_vs.dxil","outline_ps.dxil",featureRoot.Get());
        }
        if(opt.feature=="transparency"){
            makeFeatureRoot();featurePso=graphics("transparency_vs.dxil","transparency_ps.dxil",featureRoot.Get(),1,true);
            const char* shaders[]={"sort_keys_cs.dxil","sort_cs.dxil","sort_prefix_cs.dxil","sort_blocks_cs.dxil","sort_indices_cs.dxil"};
            for(size_t i=0;i<sortPso.size();++i){auto code=bytes(exeDir()/shaders[i]);D3D12_COMPUTE_PIPELINE_STATE_DESC p{};p.pRootSignature=featureRoot.Get();p.CS={code.data(),code.size()};hr(device->CreateComputePipelineState(&p,IID_PPV_ARGS(&sortPso[i])),"sort pipeline");}
            sortKeys=buffer(262144*8,D3D12_HEAP_TYPE_DEFAULT,D3D12_RESOURCE_STATE_UNORDERED_ACCESS,D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
            sortPrefix=buffer(262144*4,D3D12_HEAP_TYPE_DEFAULT,D3D12_RESOURCE_STATE_UNORDERED_ACCESS,D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
            sortBlocks=buffer(1024*4,D3D12_HEAP_TYPE_DEFAULT,D3D12_RESOURCE_STATE_UNORDERED_ACCESS,D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
            sortIndices=buffer(uint64_t(Vertices)*Cells*4,D3D12_HEAP_TYPE_DEFAULT,D3D12_RESOURCE_STATE_UNORDERED_ACCESS,D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
            begin(0);std::vector<Buffer> upload;stickerOffsets=staticBuffer(a.offsets.data(),a.offsets.size()*4,upload);submit(false);idle();
        }
        if(opt.feature=="dof"||opt.feature=="ao")makePost();
        if(opt.geometry){countPso=graphics("count_vs.dxil");auto code=bytes(exeDir()/"geometry_cs.dxil");D3D12_COMPUTE_PIPELINE_STATE_DESC p{};p.pRootSignature=root.Get();p.CS={code.data(),code.size()};hr(device->CreateComputePipelineState(&p,IID_PPV_ARGS(&computePso)),"compute pipeline");}
        else if(opt.scene=="w3"||opt.scene=="w4"||opt.scene=="w3f"){
            double slots=std::ceil(opt.duration*1000/opt.turnMs)+2;
            require(slots<double(std::numeric_limits<uint64_t>::max()/LabelBytes),"label readback size overflow");readbackSlots=uint64_t(slots);
            try{readback=buffer(readbackSlots*LabelBytes,D3D12_HEAP_TYPE_READBACK,D3D12_RESOURCE_STATE_COPY_DEST);}
            catch(const std::exception&){throw std::runtime_error("cannot allocate the run's preserved label readback buffer ("+std::to_string(readbackSlots*LabelBytes/1048576)+" MiB); no timing run started");}
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
        D3D12_ROOT_PARAMETER params[10]{};params[0].ParameterType=D3D12_ROOT_PARAMETER_TYPE_CBV;params[0].Descriptor.ShaderRegister=0;
        for(UINT i=1;i<=7;++i){params[i].ParameterType=D3D12_ROOT_PARAMETER_TYPE_SRV;params[i].Descriptor.ShaderRegister=i-1;}
        for(UINT i=8;i<=9;++i){params[i].ParameterType=D3D12_ROOT_PARAMETER_TYPE_UAV;params[i].Descriptor.ShaderRegister=i-8;}
        D3D12_ROOT_SIGNATURE_DESC d{};d.NumParameters=10;d.pParameters=params;d.Flags=D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT;
        ComPtr<ID3DBlob> blob,error;auto result=D3D12SerializeRootSignature(&d,D3D_ROOT_SIGNATURE_VERSION_1,&blob,&error);
        if(FAILED(result)&&error)throw std::runtime_error(std::string(static_cast<const char*>(error->GetBufferPointer()),error->GetBufferSize()));hr(result,"serialize root signature");
        hr(device->CreateRootSignature(0,blob->GetBufferPointer(),blob->GetBufferSize(),IID_PPV_ARGS(&root)),"CreateRootSignature");
    }
    void makeFeatureRoot(){
        D3D12_ROOT_PARAMETER params[14]{};params[0].ParameterType=D3D12_ROOT_PARAMETER_TYPE_CBV;params[0].Descriptor.ShaderRegister=0;
        for(UINT i=1;i<=7;++i){params[i].ParameterType=D3D12_ROOT_PARAMETER_TYPE_SRV;params[i].Descriptor.ShaderRegister=i-1;}
        for(UINT i=8;i<=9;++i){params[i].ParameterType=D3D12_ROOT_PARAMETER_TYPE_UAV;params[i].Descriptor.ShaderRegister=i-8;}
        params[10].ParameterType=D3D12_ROOT_PARAMETER_TYPE_SRV;params[10].Descriptor.ShaderRegister=7;
        params[11].ParameterType=D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS;params[11].Constants.ShaderRegister=1;params[11].Constants.Num32BitValues=2;
        for(UINT i=12;i<14;++i){params[i].ParameterType=D3D12_ROOT_PARAMETER_TYPE_UAV;params[i].Descriptor.ShaderRegister=i-10;}
        D3D12_ROOT_SIGNATURE_DESC d{};d.NumParameters=opt.feature=="transparency"?14:opt.feature=="outlines"?11:10;d.pParameters=params;d.Flags=D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT;
        ComPtr<ID3DBlob> blob,error;hr(D3D12SerializeRootSignature(&d,D3D_ROOT_SIGNATURE_VERSION_1,&blob,&error),"feature root serialization");
        hr(device->CreateRootSignature(0,blob->GetBufferPointer(),blob->GetBufferSize(),IID_PPV_ARGS(&featureRoot)),"feature root signature");
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
        D3D12_DESCRIPTOR_HEAP_DESC rh{};rh.Type=D3D12_DESCRIPTOR_HEAP_TYPE_RTV;rh.NumDescriptors=opt.feature=="ao"?6:4;hr(device->CreateDescriptorHeap(&rh,IID_PPV_ARGS(&rtvHeap)),"RTV heap");rtvStep=device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
        for(UINT i=0;i<(opt.geometry?1u:3u);++i)device->CreateRenderTargetView(targets[i].Get(),nullptr,rtv(i));
        D3D12_DESCRIPTOR_HEAP_DESC dh{};dh.Type=D3D12_DESCRIPTOR_HEAP_TYPE_DSV;dh.NumDescriptors=1;hr(device->CreateDescriptorHeap(&dh,IID_PPV_ARGS(&dsvHeap)),"DSV heap");D3D12_CLEAR_VALUE dc{};dc.Format=DXGI_FORMAT_D32_FLOAT;dc.DepthStencil.Depth=1;
        bool readable=opt.feature=="dof"||opt.feature=="ao";
        depth=texture(readable?DXGI_FORMAT_R32_TYPELESS:DXGI_FORMAT_D32_FLOAT,D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL,D3D12_RESOURCE_STATE_DEPTH_WRITE,opt.msaa,&dc);
        D3D12_DEPTH_STENCIL_VIEW_DESC view{};view.Format=DXGI_FORMAT_D32_FLOAT;view.ViewDimension=D3D12_DSV_DIMENSION_TEXTURE2D;
        device->CreateDepthStencilView(depth.Get(),readable?&view:nullptr,dsvHeap->GetCPUDescriptorHandleForHeapStart());
        if(opt.msaa==4){D3D12_CLEAR_VALUE cc{};cc.Format=DXGI_FORMAT_R8G8B8A8_UNORM;cc.Color[0]=0.13f;cc.Color[1]=0.145f;cc.Color[2]=0.16f;cc.Color[3]=1;msaaTarget=texture(cc.Format,D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET,D3D12_RESOURCE_STATE_RENDER_TARGET,4,&cc);device->CreateRenderTargetView(msaaTarget.Get(),nullptr,rtv(3));}
    }
    ComPtr<ID3D12PipelineState> postPipeline(const char* pixel,DXGI_FORMAT format){
        auto vs=bytes(exeDir()/"post_vs.dxil"),ps=bytes(exeDir()/pixel);D3D12_GRAPHICS_PIPELINE_STATE_DESC d{};d.pRootSignature=postRoot.Get();d.VS={vs.data(),vs.size()};d.PS={ps.data(),ps.size()};
        auto& blend=d.BlendState.RenderTarget[0];blend.SrcBlend=blend.SrcBlendAlpha=D3D12_BLEND_ONE;blend.DestBlend=blend.DestBlendAlpha=D3D12_BLEND_ZERO;blend.BlendOp=blend.BlendOpAlpha=D3D12_BLEND_OP_ADD;blend.LogicOp=D3D12_LOGIC_OP_NOOP;blend.RenderTargetWriteMask=D3D12_COLOR_WRITE_ENABLE_ALL;
        d.SampleMask=UINT_MAX;d.RasterizerState.FillMode=D3D12_FILL_MODE_SOLID;d.RasterizerState.CullMode=D3D12_CULL_MODE_NONE;d.RasterizerState.DepthClipEnable=TRUE;
        d.DepthStencilState.DepthEnable=FALSE;d.DepthStencilState.DepthFunc=D3D12_COMPARISON_FUNC_ALWAYS;
        d.DepthStencilState.StencilReadMask=D3D12_DEFAULT_STENCIL_READ_MASK;d.DepthStencilState.StencilWriteMask=D3D12_DEFAULT_STENCIL_WRITE_MASK;
        D3D12_DEPTH_STENCILOP_DESC stencil{};stencil.StencilFailOp=stencil.StencilDepthFailOp=stencil.StencilPassOp=D3D12_STENCIL_OP_KEEP;stencil.StencilFunc=D3D12_COMPARISON_FUNC_ALWAYS;d.DepthStencilState.FrontFace=d.DepthStencilState.BackFace=stencil;
        d.PrimitiveTopologyType=D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;d.NumRenderTargets=1;d.RTVFormats[0]=format;d.SampleDesc.Count=1;
        ComPtr<ID3D12PipelineState> p;hr(device->CreateGraphicsPipelineState(&d,IID_PPV_ARGS(&p)),"post-process pipeline");return p;
    }
    void makePost(){
        makeFeatureRoot();featurePso=graphics("draw_vs.dxil","draw_ps.dxil",featureRoot.Get());
        D3D12_DESCRIPTOR_RANGE range{};range.RangeType=D3D12_DESCRIPTOR_RANGE_TYPE_SRV;range.NumDescriptors=3;
        D3D12_ROOT_PARAMETER p[2]{};p[0].ParameterType=D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS;p[0].Constants.Num32BitValues=8;p[0].ShaderVisibility=D3D12_SHADER_VISIBILITY_PIXEL;
        p[1].ParameterType=D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE;p[1].DescriptorTable.NumDescriptorRanges=1;p[1].DescriptorTable.pDescriptorRanges=&range;p[1].ShaderVisibility=D3D12_SHADER_VISIBILITY_PIXEL;
        D3D12_ROOT_SIGNATURE_DESC d{};d.NumParameters=2;d.pParameters=p;d.Flags=D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT;ComPtr<ID3DBlob> blob,error;
        hr(D3D12SerializeRootSignature(&d,D3D_ROOT_SIGNATURE_VERSION_1,&blob,&error),"post root serialization");hr(device->CreateRootSignature(0,blob->GetBufferPointer(),blob->GetBufferSize(),IID_PPV_ARGS(&postRoot)),"post root signature");
        D3D12_CLEAR_VALUE clear{};clear.Format=DXGI_FORMAT_R8G8B8A8_UNORM;clear.Color[0]=0.13f;clear.Color[1]=0.145f;clear.Color[2]=0.16f;clear.Color[3]=1;
        postColour=texture(clear.Format,D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET,D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,1,&clear);device->CreateRenderTargetView(postColour.Get(),nullptr,rtv(3));
        bool ao=opt.feature=="ao";
        if(ao)for(UINT i=0;i<2;++i){D3D12_CLEAR_VALUE aux{};aux.Format=DXGI_FORMAT_R16_FLOAT;aux.Color[0]=1;postAux[i]=texture(aux.Format,D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET,D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,1,&aux);device->CreateRenderTargetView(postAux[i].Get(),nullptr,rtv(4+i));}
        D3D12_DESCRIPTOR_HEAP_DESC heap{};heap.Type=D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;heap.NumDescriptors=ao?6:3;heap.Flags=D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;hr(device->CreateDescriptorHeap(&heap,IID_PPV_ARGS(&postHeap)),"post SRV heap");postStep=device->GetDescriptorHandleIncrementSize(heap.Type);
        D3D12_SHADER_RESOURCE_VIEW_DESC srv{};srv.Shader4ComponentMapping=D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;srv.ViewDimension=D3D12_SRV_DIMENSION_TEXTURE2D;srv.Texture2D.MipLevels=1;srv.Format=clear.Format;
        auto handle=postHeap->GetCPUDescriptorHandleForHeapStart();
        for(UINT table=0;table<(ao?2u:1u);++table){
            srv.Format=clear.Format;device->CreateShaderResourceView(postColour.Get(),&srv,handle);handle.ptr+=postStep;srv.Format=DXGI_FORMAT_R32_FLOAT;device->CreateShaderResourceView(depth.Get(),&srv,handle);handle.ptr+=postStep;
            srv.Format=ao?DXGI_FORMAT_R16_FLOAT:clear.Format;device->CreateShaderResourceView(ao?postAux[table].Get():postColour.Get(),&srv,handle);handle.ptr+=postStep;
        }
        if(ao){postPso=postPipeline("ao_ps.dxil",DXGI_FORMAT_R16_FLOAT);postBlurPso=postPipeline("ao_blur_ps.dxil",DXGI_FORMAT_R16_FLOAT);postCompositePso=postPipeline("ao_composite_ps.dxil",clear.Format);}
        else postPso=postPipeline("dof_ps.dxil",clear.Format);
    }
    void postPass(ID3D12PipelineState* pipeline,D3D12_CPU_DESCRIPTOR_HANDLE target,UINT table,const Constants& c,float dx=0,float dy=0){
        list->SetPipelineState(pipeline);list->SetGraphicsRootSignature(postRoot.Get());ID3D12DescriptorHeap* heaps[]={postHeap.Get()};list->SetDescriptorHeaps(1,heaps);
        const float values[8]={float(width),float(height),c.aspect,c.zoom,5,8*float(height)/1600,dx,dy};list->SetGraphicsRoot32BitConstants(0,8,values,0);
        auto input=postHeap->GetGPUDescriptorHandleForHeapStart();input.ptr+=uint64_t(table)*3*postStep;list->SetGraphicsRootDescriptorTable(1,input);list->OMSetRenderTargets(1,&target,FALSE,nullptr);list->DrawInstanced(3,1,0,0);
    }
    void drawPost(const Constants& c,unsigned labelIndex){
        UINT back=swap->GetCurrentBackBufferIndex();transition(postColour.Get(),D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,D3D12_RESOURCE_STATE_RENDER_TARGET);
        list->SetPipelineState(featurePso.Get());bind(c,labelIndex,false,featureRoot.Get());
        D3D12_VIEWPORT viewport{0,0,float(width),float(height),0,1};D3D12_RECT rect{0,0,LONG(width),LONG(height)};list->RSSetViewports(1,&viewport);list->RSSetScissorRects(1,&rect);
        auto target=rtv(3),dsv=dsvHeap->GetCPUDescriptorHandleForHeapStart();list->OMSetRenderTargets(1,&target,FALSE,&dsv);
        const float colour[4]={0.13f,0.145f,0.16f,1};list->ClearRenderTargetView(target,colour,0,nullptr);list->ClearDepthStencilView(dsv,D3D12_CLEAR_FLAG_DEPTH,1,0,0,nullptr);list->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);list->DrawInstanced(Vertices,Cells,0,0);
        transition(postColour.Get(),D3D12_RESOURCE_STATE_RENDER_TARGET,D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE);transition(depth.Get(),D3D12_RESOURCE_STATE_DEPTH_WRITE,D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE);
        if(opt.feature=="ao"){
            transition(postAux[0].Get(),D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,D3D12_RESOURCE_STATE_RENDER_TARGET);postPass(postPso.Get(),rtv(4),1,c);transition(postAux[0].Get(),D3D12_RESOURCE_STATE_RENDER_TARGET,D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE);
            transition(postAux[1].Get(),D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,D3D12_RESOURCE_STATE_RENDER_TARGET);postPass(postBlurPso.Get(),rtv(5),0,c,1,0);transition(postAux[1].Get(),D3D12_RESOURCE_STATE_RENDER_TARGET,D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE);
            transition(postAux[0].Get(),D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,D3D12_RESOURCE_STATE_RENDER_TARGET);postPass(postBlurPso.Get(),rtv(4),1,c,0,1);transition(postAux[0].Get(),D3D12_RESOURCE_STATE_RENDER_TARGET,D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE);
        }
        transition(targets[back].Get(),D3D12_RESOURCE_STATE_PRESENT,D3D12_RESOURCE_STATE_RENDER_TARGET);postPass(opt.feature=="ao"?postCompositePso.Get():postPso.Get(),rtv(back),0,c);
        transition(targets[back].Get(),D3D12_RESOURCE_STATE_RENDER_TARGET,D3D12_RESOURCE_STATE_PRESENT);transition(depth.Get(),D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,D3D12_RESOURCE_STATE_DEPTH_WRITE);
    }
    D3D12_CPU_DESCRIPTOR_HANDLE rtv(UINT i){auto h=rtvHeap->GetCPUDescriptorHandleForHeapStart();h.ptr+=SIZE_T(i)*rtvStep;return h;}
    ComPtr<ID3D12PipelineState> graphics(const char* shader,const char* pixel="draw_ps.dxil",ID3D12RootSignature* signature=nullptr,unsigned samples=0,bool transparent=false){
        if(!samples)samples=opt.msaa;
        auto vs=bytes(exeDir()/shader),ps=bytes(exeDir()/pixel);D3D12_GRAPHICS_PIPELINE_STATE_DESC d{};d.pRootSignature=signature?signature:root.Get();d.VS={vs.data(),vs.size()};d.PS={ps.data(),ps.size()};
        auto& blend=d.BlendState.RenderTarget[0];blend.SrcBlend=D3D12_BLEND_ONE;blend.DestBlend=D3D12_BLEND_ZERO;blend.BlendOp=D3D12_BLEND_OP_ADD;blend.SrcBlendAlpha=D3D12_BLEND_ONE;blend.DestBlendAlpha=D3D12_BLEND_ZERO;blend.BlendOpAlpha=D3D12_BLEND_OP_ADD;blend.LogicOp=D3D12_LOGIC_OP_NOOP;blend.RenderTargetWriteMask=D3D12_COLOR_WRITE_ENABLE_ALL;
        if(transparent){blend.BlendEnable=TRUE;blend.SrcBlend=D3D12_BLEND_SRC_ALPHA;blend.DestBlend=D3D12_BLEND_INV_SRC_ALPHA;blend.DestBlendAlpha=D3D12_BLEND_INV_SRC_ALPHA;}
        d.SampleMask=UINT_MAX;d.RasterizerState.FillMode=D3D12_FILL_MODE_SOLID;d.RasterizerState.CullMode=D3D12_CULL_MODE_NONE;d.RasterizerState.DepthClipEnable=TRUE;d.RasterizerState.MultisampleEnable=samples==4;
        d.DepthStencilState.DepthEnable=TRUE;d.DepthStencilState.DepthWriteMask=D3D12_DEPTH_WRITE_MASK_ALL;d.DepthStencilState.DepthFunc=D3D12_COMPARISON_FUNC_LESS_EQUAL;
        if(transparent)d.DepthStencilState.DepthWriteMask=D3D12_DEPTH_WRITE_MASK_ZERO;
        d.DepthStencilState.StencilReadMask=D3D12_DEFAULT_STENCIL_READ_MASK;d.DepthStencilState.StencilWriteMask=D3D12_DEFAULT_STENCIL_WRITE_MASK;
        D3D12_DEPTH_STENCILOP_DESC stencil{};stencil.StencilFailOp=stencil.StencilDepthFailOp=stencil.StencilPassOp=D3D12_STENCIL_OP_KEEP;stencil.StencilFunc=D3D12_COMPARISON_FUNC_ALWAYS;d.DepthStencilState.FrontFace=d.DepthStencilState.BackFace=stencil;
        d.PrimitiveTopologyType=D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;d.NumRenderTargets=1;d.RTVFormats[0]=DXGI_FORMAT_R8G8B8A8_UNORM;d.DSVFormat=DXGI_FORMAT_D32_FLOAT;d.SampleDesc.Count=samples;
        ComPtr<ID3D12PipelineState> p;hr(device->CreateGraphicsPipelineState(&d,IID_PPV_ARGS(&p)),"graphics pipeline");return p;
    }
    void bind(const Constants& c,unsigned labelIndex,bool compute,ID3D12RootSignature* signature=nullptr){
        std::memcpy(active->cbData,&c,sizeof(c));
        if(!signature)signature=root.Get();
        if(compute)list->SetComputeRootSignature(signature);else list->SetGraphicsRootSignature(signature);
        auto cb=active->cb.resource->GetGPUVirtualAddress();if(compute)list->SetComputeRootConstantBufferView(0,cb);else list->SetGraphicsRootConstantBufferView(0,cb);
        for(UINT i=0;i<7;++i){auto address=(i<5?geometry[i]:i==5?labels[labelIndex]:geometry[5]).resource->GetGPUVirtualAddress();if(compute)list->SetComputeRootShaderResourceView(i+1,address);else list->SetGraphicsRootShaderResourceView(i+1,address);}
        if(signature==featureRoot.Get()&&edgeData.resource)list->SetGraphicsRootShaderResourceView(10,edgeData.resource->GetGPUVirtualAddress());
    }
    void uploadLabels(unsigned index,const std::vector<uint32_t>& values){
        std::memcpy(active->uploadData,values.data(),size_t(LabelBytes));transition(labels[index],D3D12_RESOURCE_STATE_COPY_DEST);list->CopyBufferRegion(labels[index].resource.Get(),0,active->upload.resource.Get(),0,LabelBytes);transition(labels[index],D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
    }
    void draw(const Constants& c,unsigned labelIndex,ID3D12PipelineState* pipeline,Buffer* counter=nullptr,ID3D12RootSignature* signature=nullptr){
        UINT back=opt.geometry?0:swap->GetCurrentBackBufferIndex();
        if(!opt.geometry)transition(targets[back].Get(),D3D12_RESOURCE_STATE_PRESENT,opt.msaa==4?D3D12_RESOURCE_STATE_RESOLVE_DEST:D3D12_RESOURCE_STATE_RENDER_TARGET);
        list->SetPipelineState(pipeline);bind(c,labelIndex,false,signature);
        if(counter)list->SetGraphicsRootUnorderedAccessView(8,counter->resource->GetGPUVirtualAddress());
        D3D12_VIEWPORT viewport{0,0,float(width),float(height),0,1};D3D12_RECT rect{0,0,LONG(width),LONG(height)};list->RSSetViewports(1,&viewport);list->RSSetScissorRects(1,&rect);
        auto target=rtv(opt.msaa==4?3:back);auto dsv=dsvHeap->GetCPUDescriptorHandleForHeapStart();list->OMSetRenderTargets(1,&target,FALSE,&dsv);
        const float colour[4]={0.13f,0.145f,0.16f,1};list->ClearRenderTargetView(target,colour,0,nullptr);list->ClearDepthStencilView(dsv,D3D12_CLEAR_FLAG_DEPTH,1,0,0,nullptr);list->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        if(signature&&opt.feature=="transparency"){
            D3D12_INDEX_BUFFER_VIEW indices{sortIndices.resource->GetGPUVirtualAddress(),UINT(sortIndices.size),DXGI_FORMAT_R32_UINT};list->IASetIndexBuffer(&indices);list->DrawIndexedInstanced(Vertices*Cells,1,0,0,0);
        }else list->DrawInstanced(Vertices,Cells,0,0);
        if(opt.msaa==4){
            transition(msaaTarget.Get(),D3D12_RESOURCE_STATE_RENDER_TARGET,D3D12_RESOURCE_STATE_RESOLVE_SOURCE);
            if(opt.geometry)transition(targets[0].Get(),D3D12_RESOURCE_STATE_RENDER_TARGET,D3D12_RESOURCE_STATE_RESOLVE_DEST);
            list->ResolveSubresource(targets[back].Get(),0,msaaTarget.Get(),0,DXGI_FORMAT_R8G8B8A8_UNORM);transition(msaaTarget.Get(),D3D12_RESOURCE_STATE_RESOLVE_SOURCE,D3D12_RESOURCE_STATE_RENDER_TARGET);
            if(opt.geometry)transition(targets[0].Get(),D3D12_RESOURCE_STATE_RESOLVE_DEST,D3D12_RESOURCE_STATE_RENDER_TARGET);
        }
        if(!opt.geometry)transition(targets[back].Get(),opt.msaa==4?D3D12_RESOURCE_STATE_RESOLVE_DEST:D3D12_RESOURCE_STATE_RENDER_TARGET,D3D12_RESOURCE_STATE_PRESENT);
    }
    void uavBarrier(Buffer& buffer){D3D12_RESOURCE_BARRIER b{};b.Type=D3D12_RESOURCE_BARRIER_TYPE_UAV;b.UAV.pResource=buffer.resource.Get();list->ResourceBarrier(1,&b);}
    void sort(const Constants& c,unsigned labelIndex){
        bind(c,labelIndex,true,featureRoot.Get());
        list->SetComputeRootShaderResourceView(10,stickerOffsets.resource->GetGPUVirtualAddress());
        list->SetComputeRootUnorderedAccessView(8,sortKeys.resource->GetGPUVirtualAddress());list->SetComputeRootUnorderedAccessView(9,sortIndices.resource->GetGPUVirtualAddress());
        list->SetComputeRootUnorderedAccessView(12,sortPrefix.resource->GetGPUVirtualAddress());list->SetComputeRootUnorderedAccessView(13,sortBlocks.resource->GetGPUVirtualAddress());
        transition(sortIndices,D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
        // Order the previous frame's UAV reads before this frame overwrites them.
        uavBarrier(sortKeys);uavBarrier(sortPrefix);uavBarrier(sortBlocks);
        list->SetPipelineState(sortPso[0].Get());list->Dispatch(1024,1,1);uavBarrier(sortKeys);
        list->SetPipelineState(sortPso[1].Get());
        for(UINT k=2;k<=262144;k*=2)for(UINT j=k/2;j;j/=2){UINT params[2]={k,j};list->SetComputeRoot32BitConstants(11,2,params,0);list->Dispatch(512,1,1);uavBarrier(sortKeys);}
        list->SetPipelineState(sortPso[2].Get());list->Dispatch(1024,1,1);uavBarrier(sortPrefix);uavBarrier(sortBlocks);
        list->SetPipelineState(sortPso[3].Get());list->Dispatch(1,1,1);uavBarrier(sortBlocks);
        list->SetPipelineState(sortPso[4].Get());list->Dispatch(Stickers,Cells,1);transition(sortIndices,D3D12_RESOURCE_STATE_INDEX_BUFFER);
    }
    void drawFeature(Constants c,unsigned labelIndex){
        if(opt.feature=="dof"||opt.feature=="ao"){drawPost(c,labelIndex);return;}
        if(opt.feature=="no-gaps")c.ss=1;
        if(opt.feature=="transparency")sort(c,labelIndex);
        draw(c,labelIndex,featurePso?featurePso.Get():drawPso.Get(),nullptr,featurePso?featureRoot.Get():nullptr);
    }
    Json checkEffect(const Assets& a){
        auto c=constants(a,identity(),float(width)/float(height),0);
        ComPtr<ID3D12Resource> oneDepth;ComPtr<ID3D12DescriptorHeap> oneDsv;ComPtr<ID3D12PipelineState> onePso;
        if(opt.msaa==4){
            onePso=graphics("draw_vs.dxil","draw_ps.dxil",nullptr,1);
            D3D12_CLEAR_VALUE clear{};clear.Format=DXGI_FORMAT_D32_FLOAT;clear.DepthStencil.Depth=1;
            oneDepth=texture(clear.Format,D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL,D3D12_RESOURCE_STATE_DEPTH_WRITE,1,&clear);
            D3D12_DESCRIPTOR_HEAP_DESC d{};d.Type=D3D12_DESCRIPTOR_HEAP_TYPE_DSV;d.NumDescriptors=1;hr(device->CreateDescriptorHeap(&d,IID_PPV_ARGS(&oneDsv)),"effect-check DSV heap");device->CreateDepthStencilView(oneDepth.Get(),nullptr,oneDsv->GetCPUDescriptorHandleForHeapStart());
        }
        UINT back=swap->GetCurrentBackBufferIndex();auto desc=targets[back]->GetDesc();D3D12_PLACED_SUBRESOURCE_FOOTPRINT footprint{};uint64_t size=0;
        device->GetCopyableFootprints(&desc,0,1,0,&footprint,nullptr,nullptr,&size);
        auto read=buffer(size,D3D12_HEAP_TYPE_READBACK,D3D12_RESOURCE_STATE_COPY_DEST);
        Buffer sortRead;if(opt.feature=="transparency")sortRead=buffer(uint64_t(Slots)*sizeof(SortKey),D3D12_HEAP_TYPE_READBACK,D3D12_RESOURCE_STATE_COPY_DEST);
        auto capture=[&](bool enabled){
            begin(0);
            if(!enabled&&opt.msaa==4){
                transition(targets[back].Get(),D3D12_RESOURCE_STATE_PRESENT,D3D12_RESOURCE_STATE_RENDER_TARGET);
                list->SetPipelineState(onePso.Get());bind(c,0,false);
                D3D12_VIEWPORT v{0,0,float(width),float(height),0,1};D3D12_RECT r{0,0,LONG(width),LONG(height)};list->RSSetViewports(1,&v);list->RSSetScissorRects(1,&r);
                auto colour=rtv(back),ds=oneDsv->GetCPUDescriptorHandleForHeapStart();list->OMSetRenderTargets(1,&colour,FALSE,&ds);
                const float clear[4]={0.13f,0.145f,0.16f,1};list->ClearRenderTargetView(colour,clear,0,nullptr);list->ClearDepthStencilView(ds,D3D12_CLEAR_FLAG_DEPTH,1,0,0,nullptr);list->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);list->DrawInstanced(Vertices,Cells,0,0);
                transition(targets[back].Get(),D3D12_RESOURCE_STATE_RENDER_TARGET,D3D12_RESOURCE_STATE_PRESENT);
            }else if(enabled)drawFeature(c,0);else draw(c,0,drawPso.Get());
            if(enabled&&sortRead.resource){transition(sortKeys,D3D12_RESOURCE_STATE_COPY_SOURCE);list->CopyBufferRegion(sortRead.resource.Get(),0,sortKeys.resource.Get(),0,sortRead.size);transition(sortKeys,D3D12_RESOURCE_STATE_UNORDERED_ACCESS);}
            transition(targets[back].Get(),D3D12_RESOURCE_STATE_PRESENT,D3D12_RESOURCE_STATE_COPY_SOURCE);
            D3D12_TEXTURE_COPY_LOCATION dst{};dst.pResource=read.resource.Get();dst.Type=D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;dst.PlacedFootprint=footprint;
            D3D12_TEXTURE_COPY_LOCATION src{};src.pResource=targets[back].Get();src.Type=D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;list->CopyTextureRegion(&dst,0,0,0,&src,nullptr);
            transition(targets[back].Get(),D3D12_RESOURCE_STATE_COPY_SOURCE,D3D12_RESOURCE_STATE_PRESENT);submit(false);idle();
            void* mapped=nullptr;D3D12_RANGE range{0,SIZE_T(size)};hr(read.resource->Map(0,&range,&mapped),"effect image readback");
            std::vector<uint8_t> rgba(size_t(width)*height*4);for(UINT y=0;y<height;++y)std::memcpy(rgba.data()+size_t(y)*width*4,static_cast<uint8_t*>(mapped)+footprint.Offset+size_t(y)*footprint.Footprint.RowPitch,size_t(width)*4);
            D3D12_RANGE none{0,0};read.resource->Unmap(0,&none);return rgba;
        };
        auto off=capture(false),on=capture(true);auto result=effectCheck(off,on);
        if(sortRead.resource){
            void* data=nullptr;D3D12_RANGE range{0,SIZE_T(sortRead.size)};hr(sortRead.resource->Map(0,&range,&data),"sorted key readback");sortResult=sortCheck(std::span(static_cast<const SortKey*>(data),size_t(Slots)),Slots);D3D12_RANGE none{0,0};sortRead.resource->Unmap(0,&none);
            std::cout<<"sort check: "<<sortResult.at("status").string()<<", permutation="<<sortResult.at("permutation").dump()<<", back_to_front="<<sortResult.at("back_to_front").dump()<<'\n';
        }
        std::cout<<"feature effect: "<<result.at("status").string()<<", changed pixels="<<result.at("changed_pixels").integer()<<'\n';
        if(!opt.snapshot.empty()){
            // A failed check must still end with exit 4: a snapshot error then only adds a message.
            bool failed=result.at("status").string()!="pass"||(!sortResult.null()&&sortResult.at("status").string()!="pass");
            try{fs::create_directories(opt.snapshot);require(!fs::exists(opt.snapshot/"feature-off.png")&&!fs::exists(opt.snapshot/"feature-on.png"),"snapshot files already exist");png(opt.snapshot/"feature-off.png",width,height,off);png(opt.snapshot/"feature-on.png",width,height,on);}
            catch(const std::exception& e){if(!failed)throw;std::cerr<<"sb_probe: snapshot not written: "<<e.what()<<'\n';}
        }
        return result;
    }
    void copyLabels(unsigned actuallyBound,uint64_t slot){
        uint64_t chunk=slot/readbackSlots;
        require(chunk==0||opt.scene=="w3f","preserved label readback capacity exceeded");
        while(chunk>extraReadbacks.size())extraReadbacks.push_back(buffer(readback.size,D3D12_HEAP_TYPE_READBACK,D3D12_RESOURCE_STATE_COPY_DEST));
        auto& target=chunk?extraReadbacks[size_t(chunk-1)]:readback;auto& bound=labels[actuallyBound];transition(bound,D3D12_RESOURCE_STATE_COPY_SOURCE);
        list->CopyBufferRegion(target.resource.Get(),slot%readbackSlots*LabelBytes,bound.resource.Get(),0,LabelBytes);transition(bound,D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
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
            {"power_samples",Json::Object{{"samples",conditionSamples},{"mains",samplesMains},{"battery",samplesBattery}}},{"vsync",opt.vsync},{"tearing",tearing},{"adapter",adapterName},{"driver",driver},{"msaa",opt.msaa},{"warp",opt.warp}};
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
    DisplayRequest displayRequest;
    fs::create_directories(opt.out);require(!fs::exists(opt.out/"run.json")&&!fs::exists(opt.out/"trace.jsonl"),"output directory already contains a run; choose a fresh --out");
    std::string build=buildIdentity();std::vector<uint32_t> dummySample{0};Gpu gpu(a,opt,dummySample);
    Json effect;
    if(opt.feature!="none"){
        effect=gpu.checkEffect(a);
        if(!gpu.sortResult.null()&&gpu.sortResult.at("status").string()!="pass"){std::cerr<<"sort check failed: sticker ids are not a permutation or keys are not back to front; trace not started\n";return 4;}
        if(effect.at("status").string()!="pass"){std::cerr<<"feature effect failed: no visible change above the 0.1% threshold; trace not started\n";return 4;}
    }
    bool labelScene=opt.scene=="w3"||opt.scene=="w4"||opt.scene=="w3f";auto labels=a.oracle[0];uint64_t authoritative=0;
    unsigned expected=0,bound=0;bool injectionApplied=false;std::optional<unsigned> pending;
    LabelRecords records;std::vector<Trace> trace;trace.reserve(size_t(opt.duration*200));
    Matrix q=identity();int64_t duration=turnTicks(opt.turnMs,gpu.frequency),start=0,stop=0,renderStart=qpc(),nextSample=0;uint64_t serial=0,camera=0;
    bool running=true;MSG message{};
    while(running){
        while(PeekMessageW(&message,nullptr,0,0,PM_REMOVE)){if(message.message==WM_QUIT)running=false;TranslateMessage(&message);DispatchMessageW(&message);}
        if(!running)break;
        gpu.begin(serial);
        // This read is after the previous Present returned and after the in-flight wait,
        // before this frame's upload, draw and Present. It is the single trace clock.
        int64_t now=qpc();if(!start&&double(now-renderStart)/double(gpu.frequency)>=opt.preroll){start=now;gpu.foregroundAtStart=GetForegroundWindow()==gpu.hwnd;}
        bool timed=start!=0;if(timed&&double(now-start)/double(gpu.frequency)>=opt.duration){stop=now;break;}
        if(timed&&now>=nextSample){gpu.sampleConditions();nextSample=now+gpu.frequency/10;}
        uint64_t frameIndex=uint64_t(trace.size());Turn turn{0,0,0};FrameState state{};
        if(timed){if(opt.scene=="w3f"){state=frameState(frameIndex,opt.turnFrames,opt.cycleFrames,a.angle);turn=state.turn;}else turn=turnAt(now,start,duration,a.angle);}
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
        if(opt.scene=="w2"||opt.scene=="w3"){rotate(q,0,3,0.002);++camera;}
        if(opt.scene=="w3f"&&timed){camera=state.camera;if(camera==1)q=identity();rotate(q,0,3,0.002);}
        float theta=(opt.scene=="w3"||opt.scene=="w3f")&&timed?float(turn.theta):0;
        auto c=constants(a,q,float(gpu.width)/float(gpu.height),theta);
        if(opt.feature=="none")gpu.draw(c,bound,gpu.drawPso.Get());else gpu.drawFeature(c,bound);
        if(copy){auto slot=uint64_t(records.copies.size());gpu.copyLabels(bound,slot);records.copies.push_back({frameIndex,gpu.labelRevision[bound],uint64_t(bound+1),uint64_t(expected+1),slot});}
        if(timed)trace.push_back({frameIndex,now,labelScene?gpu.labelRevision[bound]:0,opt.scene=="w3"||opt.scene=="w3f",turn.index,turn.phase,camera});
        gpu.sampleMemory();gpu.submit(true);++serial;
    }
    if(!start)start=qpc();if(!stop)stop=qpc();gpu.idle();
    Json check;
    if(labelScene){
        std::vector<const uint32_t*> mapped;std::vector<Buffer*> buffers{&gpu.readback};for(auto& b:gpu.extraReadbacks)buffers.push_back(&b);
        for(auto b:buffers){void* p=nullptr;D3D12_RANGE range{0,SIZE_T(b->size)};hr(b->resource->Map(0,&range,&p),"map preserved label readbacks after trace");mapped.push_back(static_cast<const uint32_t*>(p));}
        check=checkLabels(a,records,[&](const Copy& c){return std::span(mapped[size_t(c.slot/gpu.readbackSlots)]+c.slot%gpu.readbackSlots*Slots,size_t(Slots));});D3D12_RANGE none{0,0};for(auto b:buffers)b->resource->Unmap(0,&none);
        if(!opt.inject.empty()&&!injectionApplied){check["status"]="fail";check["injection_not_reached"]=true;}
        std::cout<<"label check: "<<check.at("status").string()<<", copies="<<records.copies.size()<<'\n';
    }
    Options written=opt;if(written.runId.empty())written.runId="sb-"+opt.scene+'-'+std::to_string(GetCurrentProcessId())+'-'+std::to_string(start);
    Json run=runJson(written,gpu.frequency,start,stop,build,gpu.environment(),check,gpu.vramPeak);run["frames"]=uint64_t(trace.size());
    if(opt.feature!="none")run["feature_effect"]=effect;
    if(!gpu.sortResult.null())run["sort_check"]=gpu.sortResult;
    bool visible=gpu.samplesNotVisible==0,foreground=gpu.samplesNotForeground==0;
    run["window"]=Json::Object{{"topmost",true},{"display_required",true},{"foreground_at_trace_start",gpu.foregroundAtStart},{"sample_period_ms",100},{"samples",gpu.conditionSamples},
        {"samples_not_visible",gpu.samplesNotVisible},{"samples_covered",gpu.samplesCovered},{"samples_not_foreground",gpu.samplesNotForeground},{"visible_throughout",visible},{"foreground_throughout",foreground},{"presents",gpu.presents}};run["injection_applied"]=injectionApplied;
    writeText(opt.out/"run.json",run.dump()+'\n');std::ostringstream lines;for(const auto& entry:trace)lines<<entry.json().dump()<<'\n';writeText(opt.out/"trace.jsonl",lines.str());
    selfTiming(trace,gpu.frequency,start,stop);
    const auto& environment=run.at("environment");
    std::cout<<"window: foreground at trace start "<<(gpu.foregroundAtStart?"yes":"no")<<", samples "<<gpu.conditionSamples<<", not visible "<<gpu.samplesNotVisible<<" (covered "<<gpu.samplesCovered<<"), not foreground "<<gpu.samplesNotForeground<<'\n';
    std::cout<<"conditions: power "<<environment.at("power_source").string()<<", power mode "<<environment.at("power_mode").string()<<", "<<environment.at("presenting_adapter").string()<<std::endl;
    // Exit 2: label check failed. Exit 3: in some sample of the trace the window was hidden,
    // minimised, cloaked, covered or not in the foreground, so the run does not represent
    // visible full-screen rendering and is no gate evidence.
    if(labelScene&&check.at("status").string()=="fail")return 2;
    return visible&&foreground?0:3;
}
}
