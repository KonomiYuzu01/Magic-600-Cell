// Agent-operated real product controls with injected router/mouse events.
// This checks GDI feedback and state boundaries, not physical typing or GPU latency.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;

internal static class KeyboardFeedbackNativeChecks {
 static T Field<T>(object instance,string name){
  for(Type type=instance.GetType();type!=null;type=type.BaseType){var field=type.GetField(name,BindingFlags.Instance|BindingFlags.NonPublic|BindingFlags.DeclaredOnly);if(field!=null)return (T)field.GetValue(instance);}
  throw new MissingFieldException(instance.GetType().Name,name);
 }
 static bool Pressed(Button button){for(Type type=button.GetType();type!=null;type=type.BaseType){var property=type.GetProperty("Pressed",BindingFlags.Instance|BindingFlags.NonPublic|BindingFlags.DeclaredOnly);if(property!=null)return (bool)property.GetValue(button,null);}throw new MissingMemberException(button.GetType().Name,"Pressed");}
 static Dictionary<string,object> Map(object value){return (Dictionary<string,object>)value;}
 static string Json(object value){return new JavaScriptSerializer().Serialize(value);}
 static Bitmap Paint(Control control){var image=new Bitmap(control.Width,control.Height);control.DrawToBitmap(image,new Rectangle(Point.Empty,image.Size));return image;}
 static int ChangedPixels(Bitmap first,Bitmap second){
  if(first.Size!=second.Size)return Int32.MaxValue;
  int count=0;for(int y=0;y<first.Height;y++)for(int x=0;x<first.Width;x++)if(first.GetPixel(x,y)!=second.GetPixel(x,y))count++;return count;
 }
 static void MouseDown(Button button){button.GetType().GetMethod("OnMouseDown",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(button,new object[]{new MouseEventArgs(MouseButtons.Left,1,button.Width/2,button.Height/2,0)});}
 static async Task Change(Func<Dictionary<string,object>,Task<bool>> send,Func<Task> ready,Action<bool,string> check,Dictionary<string,object> payload){
  check(await send(payload),"Feedback fixture accepted explicit "+Convert.ToString(payload["action"]));await ready();
 }
 internal static async Task Run(ExperimentShell shell,Form host,Func<Task> ready,Func<Dictionary<string,object>,Task<bool>> send,Action<bool,string> check,Action<string,Form> image){
  var input=Field<ExperimentInput>(shell,"input");var panel=Field<FlowLayoutPanel>(shell,"keyboard");
  var windows=Field<Dictionary<string,Form>>(shell,"workWindows");var keyboard=windows["keyboard"];
  var api=Field<LocalApi>(shell,"api");var workspace=Map(shell.Work["workspace"]);
  string originalBank=Convert.ToString(workspace["bank"]),originalMode=Convert.ToString(Field<ComboBox>(shell,"gripMode").SelectedItem);
  string initialHash=Convert.ToString(shell.Work["hash"]),draft=Json(workspace["draft"]),next=Json(workspace["next"]),current=Json(shell.Work["current"]);
  byte[] labels=await Task.Factory.StartNew(()=>api.Bytes("labels"));
  var formRegion=host.Region;var windowRegion=keyboard.Region;var scroll=panel.AutoScrollPosition;
  string gripCode=null,twistCode=null;Button gripKey=null,twistKey=null;Region gripRegion=null,twistRegion=null;
  Func<ExperimentInputState> state=()=> (ExperimentInputState)typeof(ExperimentShell).GetMethod("InputState",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(shell,null);
  Func<string,Button> key=code=>Field<Dictionary<Button,string>>(shell,"physicalKeyboardKeys").Single(pair=>pair.Value==code).Key;
  Action activate=()=>{keyboard.Activate();panel.Focus();};
  Action<string> down=code=>input.HandleKeyDown(code,false,false,false,false,false);
  Action<string> up=code=>input.HandleKeyUp(code);
  Exception failure=null;
  try{
   check(keyboard.Visible&&keyboard.IsHandleCreated,"Feedback uses the visible product Keyboard window");
   await Change(send,ready,check,LocalApi.D("action","settings","view",LocalApi.D("grip_mode","hold")));activate();input.Reset("Feedback fixture starts with released input.");
   var keys=state();var captured=keys.GripKeys.FirstOrDefault(pair=>pair.Value>=1&&pair.Value<=600);
   check(captured.Key!=null,"Feedback fixture has an explicitly captured legal cap");gripCode=captured.Key;int cap=captured.Value;
   twistCode=keys.TwistKeys.Keys.First(code=>!keys.CommandKeys.ContainsKey(code));gripKey=key(gripCode);twistKey=key(twistCode);
   gripRegion=gripKey.Region;twistRegion=twistKey.Region;
   panel.ScrollControlIntoView(gripKey);gripKey.Update();
   using(var released=Paint(gripKey)){
    down(gripCode);
    check(Field<bool>(gripKey,"PhysicalPressed")&&Pressed(gripKey)&&Field<bool>(gripKey,"SelectedGrip"),"Physical Grip down immediately paints press and selected cap before awaiting any command");
    check(input.ActiveGripCell==cap&&!Field<bool>(gripKey,"RejectedPress"),"Held Grip uses its exact captured cap without a rejection mark");
    using(var held=Paint(gripKey))check(ChangedPixels(released,held)>20,"Held Grip visibly changes the actual product key pixels");
    image("keyboard-feedback-01-held",keyboard);
    up(gripCode);
    check(!Pressed(gripKey)&&!Field<bool>(gripKey,"SelectedGrip")&&input.ActiveGripCell==null,"Hold release immediately clears both press and active Grip");
    image("keyboard-feedback-02-released",keyboard);
   }

   panel.ScrollControlIntoView(twistKey);
   using(var idle=Paint(twistKey)){
    down(twistCode);
    check(Field<bool>(twistKey,"PhysicalPressed")&&Field<bool>(twistKey,"RejectedPress"),"Twist without a Grip displays a rejected press on its actual key");
    check(!input.Pending,"Rejected Twist created no asynchronous turn");
    using(var rejected=Paint(twistKey))check(ChangedPixels(idle,rejected)>20,"Rejected press has visible pixel feedback even when the key is disabled");
    image("keyboard-feedback-03-rejected",keyboard);up(twistCode);
    check(!Pressed(twistKey)&&!Field<bool>(twistKey,"RejectedPress"),"Rejected key-up clears the momentary rejection mark");
   }

   await Change(send,ready,check,LocalApi.D("action","settings","view",LocalApi.D("grip_mode","latch")));activate();panel.ScrollControlIntoView(gripKey);
   down(gripCode);up(gripCode);
   check(!Pressed(gripKey)&&Field<bool>(gripKey,"SelectedGrip")&&input.ActiveGripCell==cap,"Latch keeps the selected Grip outline after the physical press ends");
   image("keyboard-feedback-04-latched",keyboard);
   down(gripCode);up(gripCode);
   check(input.ActiveGripCell==null&&!Field<bool>(gripKey,"SelectedGrip"),"A fresh explicit Latch press releases the selected cap");

   await Change(send,ready,check,LocalApi.D("action","settings","view",LocalApi.D("grip_mode","hold")));activate();panel.ScrollControlIntoView(gripKey);
   // No click is injected here: capture-loss must clear only the painted depression.
   gripKey.Capture=true;MouseDown(gripKey);
   check(gripKey.Capture&&Pressed(gripKey)&&!Field<bool>(gripKey,"PhysicalPressed"),"Onscreen mouse down gives independent momentary press feedback");
   image("keyboard-feedback-05-pointer-down",keyboard);
   gripKey.Capture=false;await Task.Delay(20);
   check(!Pressed(gripKey)&&input.ActiveGripCell==null,"Losing mouse capture clears onscreen depression without selecting or turning");
   gripKey.PerformClick();
   check(input.ActiveGripCell==cap&&Field<bool>(gripKey,"SelectedGrip")&&!Pressed(gripKey),"Explicit onscreen Grip click selects its cap without pretending a physical key is held");
   gripKey.PerformClick();check(input.ActiveGripCell==null,"Second explicit onscreen Grip click releases the cap");

   activate();down(gripCode);host.Activate();await Task.Delay(40);
   check(Form.ActiveForm==host,"Focus-loss fixture really activated the main native window");
   check(!Pressed(gripKey)&&input.ActiveGripCell==null,"Owned-window deactivation clears physical press and Grip");
   up(gripCode);activate();

   down(gripCode);
   string otherBank=originalBank=="33-B"?"33-A":"33-B";
   await Change(send,ready,check,LocalApi.D("action","bank","id",otherBank));activate();
   check(!input.IsVisuallyPressed(gripCode)&&input.ActiveGripCell==null,"Explicit bank adoption clears old held-key presentation and active cap");
   down(gripCode);
   check(input.IsVisuallyRejected(gripCode)&&input.ActiveGripCell==null,"Unreleased Grip cannot acquire the new bank's meaning");
   up(gripCode);
   await Change(send,ready,check,LocalApi.D("action","bank","id",originalBank));activate();

   input.HandleKeyDown(gripCode,false,false,false,false,true);
   check(!input.IsVisuallyPressed(gripCode)&&input.ActiveGripCell==null,"Text-owned injected key is not shown as an application operation");
   input.HandleKeyUp(gripCode,true);
   check(Object.ReferenceEquals(gripRegion,gripKey.Region)&&Object.ReferenceEquals(twistRegion,twistKey.Region)&&Object.ReferenceEquals(formRegion,host.Region)&&Object.ReferenceEquals(windowRegion,keyboard.Region),"Feedback and rounded-key paint leave all captured window and key Regions unchanged");
   check(Convert.ToString(shell.Work["hash"])==initialHash&&Json(Map(shell.Work["workspace"])["draft"])==draft&&Json(Map(shell.Work["workspace"])["next"])==next&&Json(shell.Work["current"])==current,"Key presentation, rejected inputs and explicit input settings preserve draft, Current and locked Next");
   var finalLabels=await Task.Factory.StartNew(()=>api.Bytes("labels"));check(labels.SequenceEqual(finalLabels),"Keyboard feedback checks preserve all 259800 authoritative labels");
  }catch(Exception error){failure=error;}finally{
   if(gripKey!=null&&!gripKey.IsDisposed)gripKey.Capture=false;
   if(gripCode!=null)up(gripCode);if(twistCode!=null)up(twistCode);input.Reset("Feedback fixture ended; release input.");
  }
  try{
   if(Convert.ToString(Map(shell.Work["workspace"])["bank"])!=originalBank){if(!await send(LocalApi.D("action","bank","id",originalBank)))throw new InvalidOperationException("Could not restore feedback fixture bank.");await ready();}
   if(Convert.ToString(Field<ComboBox>(shell,"gripMode").SelectedItem)!=originalMode){if(!await send(LocalApi.D("action","settings","view",LocalApi.D("grip_mode",originalMode))))throw new InvalidOperationException("Could not restore feedback fixture Grip mode.");await ready();}
  }catch(Exception restoreError){if(failure!=null)throw new AggregateException("Keyboard feedback check and context restoration both failed.",failure,restoreError);throw;}finally{
   if(!panel.IsDisposed)panel.AutoScrollPosition=new Point(-scroll.X,-scroll.Y);
   if(!keyboard.IsDisposed&&keyboard.Visible)activate();
  }
  if(failure!=null)throw new InvalidOperationException("Keyboard feedback check failed: "+failure.Message,failure);
 }
}
