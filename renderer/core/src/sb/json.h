#pragma once
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <iomanip>
#include <map>
#include <sstream>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <variant>
#include <vector>

// The probe needs only JSON values, not a third-party runtime dependency.
struct Json {
    using Array = std::vector<Json>;
    using Object = std::map<std::string, Json>;
    std::variant<std::nullptr_t, bool, double, std::string, Array, Object> value = nullptr;
    Json() = default;
    Json(std::nullptr_t) {}
    Json(bool v) : value(v) {}
    Json(double v) : value(v) {}
    Json(float v) : value(double(v)) {}
    template<class T, std::enable_if_t<std::is_integral_v<T> && !std::is_same_v<T,bool>, int> = 0>
    Json(T v) : value(double(v)) {}
    Json(const char* v) : value(std::string(v)) {}
    Json(std::string v) : value(std::move(v)) {}
    Json(Array v) : value(std::move(v)) {}
    Json(Object v) : value(std::move(v)) {}
    const Object& object() const { return std::get<Object>(value); }
    const Array& array() const { return std::get<Array>(value); }
    const std::string& string() const { return std::get<std::string>(value); }
    double number() const { return std::get<double>(value); }
    int64_t integer() const {
        double n = number();
        if (!std::isfinite(n) || std::floor(n) != n || n < -9e15 || n > 9e15)
            throw std::runtime_error("JSON integer out of range");
        return int64_t(n);
    }
    const Json& at(const std::string& key) const { return object().at(key); }
    Json& operator[](const std::string& key) { return std::get<Object>(value)[key]; }
    bool null() const { return std::holds_alternative<std::nullptr_t>(value); }
    static std::string quote(const std::string& s) {
        std::ostringstream out; out << '"';
        for (unsigned char ch : s) {
            switch (ch) {
            case '"': out << "\\\""; break;
            case '\\': out << "\\\\"; break;
            case '\n': out << "\\n"; break;
            case '\r': out << "\\r"; break;
            case '\t': out << "\\t"; break;
            default:
                if (ch < 32) out << "\\u" << std::hex << std::setw(4) << std::setfill('0') << unsigned(ch) << std::dec;
                else out << ch;
            }
        }
        out << '"'; return out.str();
    }
    std::string dump() const {
        if (null()) return "null";
        if (auto b = std::get_if<bool>(&value)) return *b ? "true" : "false";
        if (auto n = std::get_if<double>(&value)) {
            if (!std::isfinite(*n)) throw std::runtime_error("nonfinite JSON number");
            std::ostringstream out; out << std::setprecision(17) << *n; return out.str();
        }
        if (auto s = std::get_if<std::string>(&value)) return quote(*s);
        std::string out;
        if (auto a = std::get_if<Array>(&value)) {
            out = "[";
            for (const auto& v : *a) { if (out.size() > 1) out += ','; out += v.dump(); }
            return out + ']';
        }
        out = "{";
        for (const auto& [k, v] : object()) { if (out.size() > 1) out += ','; out += quote(k) + ':' + v.dump(); }
        return out + '}';
    }
    static Json parse(const std::string& text) {
        struct Reader {
            const std::string& s; size_t p = 0;
            [[noreturn]] void bad() const { throw std::runtime_error("invalid JSON at byte " + std::to_string(p)); }
            void ws() { while (p < s.size() && (s[p]==' ' || s[p]=='\r' || s[p]=='\n' || s[p]=='\t')) ++p; }
            bool take(char c) { ws(); if (p < s.size() && s[p] == c) { ++p; return true; } return false; }
            std::string str() {
                if (!take('"')) bad();
                std::string out;
                while (p < s.size()) {
                    unsigned char c = s[p++];
                    if (c == '"') return out;
                    if (c < 32) bad();
                    if (c != '\\') { out += char(c); continue; }
                    if (p == s.size()) bad();
                    switch (s[p++]) {
                    case '"': out += '"'; break; case '\\': out += '\\'; break; case '/': out += '/'; break;
                    case 'b': out += '\b'; break; case 'f': out += '\f'; break;
                    case 'n': out += '\n'; break; case 'r': out += '\r'; break; case 't': out += '\t'; break;
                    case 'u': {
                        unsigned u = 0;
                        for (int i=0; i<4; ++i) {
                            if (p == s.size()) bad(); char h=s[p++];
                            int x = h>='0'&&h<='9'?h-'0':h>='a'&&h<='f'?h-'a'+10:h>='A'&&h<='F'?h-'A'+10:-1;
                            if (x<0) bad(); u=u*16+unsigned(x);
                        }
                        // Input assets are UTF-8; escaped surrogate pairs are decoded as UTF-8 too.
                        if (u>=0xd800 && u<=0xdbff) {
                            if (p+6>s.size() || s[p++]!='\\' || s[p++]!='u') bad();
                            unsigned low=0;
                            for (int i=0;i<4;++i) { char h=s[p++]; int x=h>='0'&&h<='9'?h-'0':h>='a'&&h<='f'?h-'a'+10:h>='A'&&h<='F'?h-'A'+10:-1; if(x<0)bad(); low=low*16+unsigned(x); }
                            if(low<0xdc00 || low>0xdfff)bad(); u=0x10000+(u-0xd800)*1024+low-0xdc00;
                        } else if (u>=0xdc00 && u<=0xdfff) bad();
                        if(u<128)out+=char(u);
                        else if(u<2048){out+=char(0xc0|(u>>6));out+=char(0x80|(u&63));}
                        else if(u<65536){out+=char(0xe0|(u>>12));out+=char(0x80|((u>>6)&63));out+=char(0x80|(u&63));}
                        else {out+=char(0xf0|(u>>18));out+=char(0x80|((u>>12)&63));out+=char(0x80|((u>>6)&63));out+=char(0x80|(u&63));}
                        break;
                    }
                    default: bad();
                    }
                }
                bad();
            }
            Json read() {
                ws(); if(p==s.size())bad();
                if(s[p]=='"')return str();
                if(take('{')) {
                    Object o; if(take('}'))return o;
                    do { auto k=str(); if(!take(':'))bad(); if(!o.emplace(k,read()).second)bad(); } while(take(','));
                    if(!take('}'))bad(); return o;
                }
                if(take('[')) { Array a; if(take(']'))return a; do{a.push_back(read());}while(take(',')); if(!take(']'))bad(); return a; }
                for(const auto& [word,v] : std::vector<std::pair<std::string,Json>>{{"true",true},{"false",false},{"null",nullptr}})
                    if(s.compare(p,word.size(),word)==0){p+=word.size();return v;}
                size_t begin=p;
                if(s[p]=='-')++p;
                if(p==s.size())bad();
                if(s[p]=='0')++p;
                else {if(s[p]<'1'||s[p]>'9')bad();while(p<s.size()&&s[p]>='0'&&s[p]<='9')++p;}
                if(p<s.size()&&s[p]=='.'){++p;size_t b=p;while(p<s.size()&&s[p]>='0'&&s[p]<='9')++p;if(b==p)bad();}
                if(p<s.size()&&(s[p]=='e'||s[p]=='E')){++p;if(p<s.size()&&(s[p]=='+'||s[p]=='-'))++p;size_t b=p;while(p<s.size()&&s[p]>='0'&&s[p]<='9')++p;if(b==p)bad();}
                double n=std::strtod(s.substr(begin,p-begin).c_str(),nullptr); if(!std::isfinite(n))bad(); return n;
            }
        } r{text};
        Json out=r.read(); r.ws(); if(r.p!=text.size())r.bad(); return out;
    }
};
