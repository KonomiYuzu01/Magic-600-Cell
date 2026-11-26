import sys

root = sys.argv[1]
BS = chr(92)


def edit(name, pairs):
    p = root + "/" + name
    b = open(p, 'rb').read().decode()
    crlf = "\r\n" in b
    if crlf:
        b = b.replace("\r\n", "\n")
    for old, new in pairs:
        old = old.replace("<BS>", BS)
        new = new.replace("<BS>", BS)
        assert b.count(old) == 1, (name, old[:80])
        b = b.replace(old, new)
    if crlf:
        b = b.replace("\n", "\r\n")
    open(p, 'wb').write(b.encode())


edit("renderer/core/src/interop.cpp", [
("""        require(out_ticket != nullptr, "null ticket output");
        const auto data = sa2::preparation(preparation);
        auto& scene = product_scene(ctx);
        scene.product.check_prepare(data);
        scene.require_healthy();
        live(ctx);
        scene.product_ready(ctx->producer());
        const auto ticket = scene.product.reserve_ticket();
        const auto fence = scene.product_prepare(ticket, data, ctx->producer(), [&] { ctx->drained = false; });
        scene.product.prepared(ticket, data, fence);
        *out_ticket = ticket;
        release(ctx, scene);
        return SA2_OK;""",
"""        require(out_ticket != nullptr, "null ticket output");
        using TC = std::chrono::steady_clock;
        const auto t0 = TC::now();
        const auto data = sa2::preparation(preparation);
        const auto t1 = TC::now();
        auto& scene = product_scene(ctx);
        scene.product.check_prepare(data);
        scene.require_healthy();
        live(ctx);
        scene.product_ready(ctx->producer());
        const auto ticket = scene.product.reserve_ticket();
        const auto t2 = TC::now();
        const auto fence = scene.product_prepare(ticket, data, ctx->producer(), [&] { ctx->drained = false; });
        const auto t3 = TC::now();
        scene.product.prepared(ticket, data, fence);
        *out_ticket = ticket;
        release(ctx, scene);
        const auto t4 = TC::now();
        const auto ms = [](TC::time_point a, TC::time_point b) { return std::chrono::duration<double, std::milli>(b - a).count(); };
        std::fprintf(stderr, "TIMER conv=%.3f checks=%.3f gpu=%.3f tail=%.3f<BS>n", ms(t0, t1), ms(t1, t2), ms(t2, t3), ms(t3, t4));
        return SA2_OK;"""),
])
edit("renderer/core/src/scene.cpp", [
("""    ProductGpu::Set set;
    auto upload = p.spare_uploads.take();""",
"""    using TC = std::chrono::steady_clock;
    const auto s0 = TC::now();
    ProductGpu::Set set;
    auto upload = p.spare_uploads.take();"""),
("""    auto* bytes = g.map(set.staging);
    std::memcpy(bytes, data.visible.data(), size_t(sb::LabelBytes)); std::memcpy(bytes + sb::LabelBytes, data.instances.data(), size_t(sb::LabelBytes));
    set.staging.resource->Unmap(0, nullptr);
    auto& stored = p.sets[ticket] = std::move(set);""",
"""    const auto s1 = TC::now();
    auto* bytes = g.map(set.staging);
    std::memcpy(bytes, data.visible.data(), size_t(sb::LabelBytes)); std::memcpy(bytes + sb::LabelBytes, data.instances.data(), size_t(sb::LabelBytes));
    set.staging.resource->Unmap(0, nullptr);
    const auto s2 = TC::now();
    std::fprintf(stderr, "TIMER resources=%.3f memcpy=%.3f reused=%d<BS>n", std::chrono::duration<double, std::milli>(s1 - s0).count(),
        std::chrono::duration<double, std::milli>(s2 - s1).count(), int(bool(upload)));
    auto& stored = p.sets[ticket] = std::move(set);"""),
])
print("ok")
