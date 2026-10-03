#include "probe.h"
#include <algorithm>
#include <map>
#include <set>
namespace sb {
using Point=std::array<double,4>;
static Point minus(const Point& a,const Point& b){Point out{};for(int i=0;i<4;++i)out[i]=a[i]-b[i];return out;}
static double dot(const Point& a,const Point& b){double s=0;for(int i=0;i<4;++i)s+=a[i]*b[i];return s;}
EdgeMasks edgeMasks(const Assets& a){
    EdgeMasks out;out.triangles.resize(Vertices/3);out.perSticker.resize(Stickers);
    for(uint32_t sticker=0;sticker<Stickers;++sticker){
        std::map<Point,uint32_t> ids;std::vector<Point> points;
        auto id=[&](uint32_t vi){Point p{};for(int c=0;c<4;++c)p[c]=a.vertices[size_t(vi)*4+c];auto [it,added]=ids.emplace(p,uint32_t(points.size()));if(added)points.push_back(p);return it->second;};
        using Triangle=std::array<uint32_t,3>;using Edge=std::pair<uint32_t,uint32_t>;
        std::vector<std::pair<uint32_t,Triangle>> raw;std::set<Triangle> unique;
        std::map<Edge,std::vector<uint32_t>> opposite;
        for(uint32_t vi=a.offsets[sticker];vi<a.offsets[sticker+1];vi+=3){
            Triangle tri{id(vi),id(vi+1),id(vi+2)};
            auto canonical=tri;std::sort(canonical.begin(),canonical.end());bool first=unique.insert(canonical).second;
            if(!first)++out.duplicates;
            // Count exact repeated vertices separately from numerically collapsed
            // triangles. Both remain in the GPU draw; only adjacency ignores them.
            if(tri[0]==tri[1]||tri[1]==tri[2]||tri[2]==tri[0]){++out.degenerate;continue;}
            auto u=minus(points[tri[1]],points[tri[0]]),v=minus(points[tri[2]],points[tri[0]]);
            if(dot(u,u)*dot(v,v)-dot(u,v)*dot(u,v)<=1e-18){++out.nearZero;continue;}
            raw.emplace_back(vi/3,tri);if(!first)continue;
            for(int i=0;i<3;++i){auto edge=std::minmax(tri[(i+1)%3],tri[(i+2)%3]);opposite[edge].push_back(tri[i]);}
        }
        std::set<Edge> features;
        for(const auto& [edge,other]:opposite){
            bool feature=other.size()!=2;
            if(other.size()==1)++out.openEdges;else if(other.size()>=3)++out.multipleEdges;
            if(other.size()==2){
                auto e=minus(points[edge.second],points[edge.first]);auto u=minus(points[other[0]],points[edge.first]);auto v=minus(points[other[1]],points[edge.first]);
                double ee=dot(e,e),uu=dot(u,u),vv=dot(v,v),eu=dot(e,u),ev=dot(e,v),uv=dot(u,v);
                double gram=ee*uu*vv+2*eu*ev*uv-ee*uv*uv-uu*ev*ev-vv*eu*eu;
                feature=gram/(ee*uu*vv)>1e-6;
            }
            if(feature){features.insert(edge);++out.featureEdges;++out.perSticker[sticker];}else ++out.diagonals;
        }
        // Duplicates inherit the same geometric edges; degenerate triangles stay
        // in the draw with a zero mask. No mesh vertices or triangles are removed.
        for(const auto& [index,tri]:raw)for(int i=0;i<3;++i)if(features.contains(std::minmax(tri[(i+1)%3],tri[(i+2)%3])))out.triangles[index]|=1u<<i;
    }
    return out;
}
}
