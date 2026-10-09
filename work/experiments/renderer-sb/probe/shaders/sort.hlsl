#include "geometry.hlsl"
StructuredBuffer<uint> offsets : register(t7);
struct Key { float depth; uint id; };
RWStructuredBuffer<Key> keys : register(u0);
RWStructuredBuffer<uint> indices : register(u1);
RWStructuredBuffer<uint> prefix : register(u2);
RWStructuredBuffer<uint> blocks : register(u3);
cbuffer SortConstants : register(b1) { uint mergeSize, stride; };
groupshared uint scan[1024];
[numthreads(256,1,1)]
void keyMain(uint3 thread : SV_DispatchThreadID) {
    uint slot = thread.x;
    Key key;
    key.id = slot < 259800 ? slot : 0xffffffff;
    key.depth = -3.402823466e38;
    if (slot < 259800) {
        uint cell = slot/433;
        float4 center = centers[slot%433];
        // projectVertex at the sticker centre: its sticker-shrink offset is zero.
        float4 v = (N0 + cellShrink*(center-N0) + cellShrink*stickerShrink*(center-center))/radius;
        float4 world = frames[cell*4]*v.x + frames[cell*4+1]*v.y
                     + frames[cell*4+2]*v.z + frames[cell*4+3]*v.w;
        if (theta != 0 && animated[slot] != 0) {
            float x = dot(world,turnU), y = dot(world,turnV);
            float c = cos(theta), s = sin(theta);
            world += ((c-1)*x-s*y)*turnU + (s*x+(c-1)*y)*turnV;
        }
        world = mul(Q,world);
        key.depth = 5 - d4*world.z/(d4-world.w);
    }
    keys[slot] = key;
}
bool before(Key a, Key b) { return a.depth>b.depth || (a.depth==b.depth && a.id<b.id); }
[numthreads(256,1,1)]
void sortMain(uint3 thread : SV_DispatchThreadID) {
    uint i = (thread.x/stride)*(2*stride) + thread.x%stride, j = i+stride;
    Key a = keys[i], b = keys[j];
    bool descending = (i & mergeSize)==0;
    if (descending ? before(b,a) : before(a,b)) { keys[i]=b; keys[j]=a; }
}
[numthreads(256,1,1)]
void prefixMain(uint3 thread : SV_DispatchThreadID, uint3 group : SV_GroupID, uint t : SV_GroupIndex) {
    uint slot = keys[thread.x].id;
    uint count = slot<259800 ? offsets[slot%433+1]-offsets[slot%433] : 0;
    scan[t] = count;
    GroupMemoryBarrierWithGroupSync();
    for (uint step=1; step<256; step*=2) {
        uint add = t>=step ? scan[t-step] : 0;
        GroupMemoryBarrierWithGroupSync();
        scan[t] += add;
        GroupMemoryBarrierWithGroupSync();
    }
    prefix[thread.x] = scan[t]-count;
    if (t==255) blocks[group.x]=scan[t];
}
[numthreads(1024,1,1)]
void blocksMain(uint t : SV_GroupIndex) {
    uint count = blocks[t];
    scan[t] = count;
    GroupMemoryBarrierWithGroupSync();
    for (uint step=1; step<1024; step*=2) {
        uint add = t>=step ? scan[t-step] : 0;
        GroupMemoryBarrierWithGroupSync();
        scan[t] += add;
        GroupMemoryBarrierWithGroupSync();
    }
    blocks[t] = scan[t]-count;
}
[numthreads(256,1,1)]
void indicesMain(uint3 group : SV_GroupID, uint t : SV_GroupIndex) {
    uint rank = group.y*433+group.x;
    uint slot = keys[rank].id, local = slot%433;
    uint count = offsets[local+1]-offsets[local];
    uint first = prefix[rank]+blocks[rank/256];
    if (t<count) indices[first+t]=slot/433*30480+offsets[local]+t;
}
