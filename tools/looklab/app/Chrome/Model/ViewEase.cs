namespace Magic600.Chrome;

// A discrete view transition starts at the value last drawn. More input can change the target mid-flight.
public sealed class ViewEase
{
    private double began;
    private double[] curve = [];
    public double Drawn { get; private set; }
    public double Target { get; private set; }
    public double From { get; private set; }
    public double Seconds { get; private set; }

    public ViewEase(double initial) => Drawn = Target = From = initial;

    public void Retarget(double target, double now, double seconds, double[] ease)
    {
        (From, Target, began, Seconds, curve) = (Drawn, target, now, seconds, ease);
        if (seconds <= 0) Drawn = Target;
    }

    public bool Step(double now, bool reduced)
    {
        if (Drawn == Target) return false;
        double elapsed = now - began;
        double next = reduced || elapsed >= Seconds ? Target
            : elapsed <= 0 ? From : From + (Target - From) * MotionTable.Ease(curve, elapsed / Seconds);
        if (Drawn == next) return false;
        Drawn = next;
        return true;
    }
}
