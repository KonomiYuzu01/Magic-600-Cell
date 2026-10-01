// Isolated agent-driven recurring-method acceptance. No model writes or target search.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;

internal static class ContinuousSolverNativeChecks {
 static ExperimentShell shell; static Form host; static Func<Task> ready;
 static Action<bool,string> check; static Action<string,Form> image;
 static Func<string,Form,Task> desktop; static string home;
 static readonly JavaScriptSerializer json=new JavaScriptSerializer{MaxJsonLength=64000000};
 static Dictionary<string,object> Map(object x){return (Dictionary<string,object>)x;}
 static object[] Items(object x){return (object[])x;}
 static object Ordered(object value){var map=value as Dictionary<string,object>;if(map!=null)return new SortedDictionary<string,object>(map.ToDictionary(p=>p.Key,p=>Ordered(p.Value)));var list=value as object[];return list==null?value:list.Select(Ordered).ToArray();}
 static Dictionary<string,object> Work{get{return Map(shell.Work["workspace"]);}}
 static string Hash{get{return Convert.ToString(shell.Work["hash"]);}}
 static T Field<T>(string name){return (T)typeof(ExperimentShell).GetField(name,BindingFlags.Instance|BindingFlags.NonPublic).GetValue(shell);}
 static object Call(string name,params object[] args){return typeof(ExperimentShell).GetMethod(name,BindingFlags.Instance|BindingFlags.NonPublic).Invoke(shell,args);}
 static IEnumerable<Control> Children(Control parent){foreach(Control c in parent.Controls){yield return c;foreach(var x in Children(c))yield return x;}}
 static async Task Command(string action,params object[] args){var body=LocalApi.D("action",action);for(int i=0;i<args.Length;i+=2)body[Convert.ToString(args[i])]=args[i+1];check(await shell.Send(body),"Continuous accepted "+action);await ready();}
 static async Task SelectContext(Dictionary<string,object> m,string goal){
  await Command("focus","identity",m["identity"],"target",m["target"]);
  await Command("operation-new");await Command("next-clear");
  await Command("roles","positions",m["roles"]);await Command("goal","goal",goal);
  await Command("reference","word",new object[0]);
  if(goal!="prepare"&&Convert.ToBoolean(m["target_home"]))await Command("target-home");
  CheckBindings(m,goal);
 }
 static void CheckBindings(Dictionary<string,object> m,string goal){check(Convert.ToInt32(Work["orbit"])==Convert.ToInt32(m["orbit"])&&Convert.ToInt32(Work["current"])==Convert.ToInt32(m["identity"])&&Convert.ToInt32(Work["target"])==Convert.ToInt32(m["target"])&&Items(Work["roles"]).Select(Convert.ToInt32).SequenceEqual(Items(m["roles"]).Select(Convert.ToInt32))&&Convert.ToString(Work["goal"])==goal,"Chosen canonical orbit, identity, destination, buffers and intent are intact");}
 static string SavedWork(){return json.Serialize(Ordered(LocalApi.D("current",Work["current"],"target",Work["target"],"next",Work["next"],"roles",Work["roles"],"draft",Work["draft"],"draft_sources",Work["draft_sources"],"bank",Work["bank"],"goal",Work["goal"],"reference",Work["reference"],"ownership",shell.Work["operation_state"])));}
 static async Task Commit(bool requireGoal){
  await Command("review");var review=Map(shell.Work["review"]);
  check(Convert.ToString(review["status"])=="Ready","Complete operation passes current protection");
  if(requireGoal){var goal=Map(review["goal_result"]);check(Convert.ToString(goal["status"])=="Met"&&Object.Equals(goal["before"],false)&&Object.Equals(goal["after"],true),"Chosen operation changes explicit work intent from unmet to met");}
  string predicted=Convert.ToString(review["post_hash"]);
  await Command("preview","review_id",review["id"]);await Command("commit");
  check(Hash==predicted,"Actual full-label commit equals reviewed prediction");
  if(Hash==home)await SessionNativeChecks.CloseCompletion(shell,ready,check,image,"continuous-completion");
 }
 static async Task LoadSheet(string name){
  host.Activate();Call("FocusWorkspace");Call("ShowWorkSheet",name);
  var deadline=DateTime.UtcNow.AddSeconds(60);Form dialog=null;
  while(dialog==null){dialog=Application.OpenForms.Cast<Form>().FirstOrDefault(f=>f.Visible&&f.Text=="Reuse work sheet · "+name);if(DateTime.UtcNow>deadline)throw new TimeoutException("Work sheet did not open: "+Field<Label>("feedback").Text);await Task.Delay(20);}
  var load=Children(dialog).OfType<Button>().Single(b=>b.Text.StartsWith("Load fixed steps"));
  check(load.Enabled,"Fixed sheet has a fresh explicit comparison");load.PerformClick();await ready();
  check(shell.Work["review"]==null&&!Convert.ToBoolean(shell.Work["executable"]),"Sheet reuse does not inherit execution permission");
 }
 internal static async Task Run(ExperimentShell value,Form form,Func<Task> wait,Action<bool,string> assertion,Action<string,Form> capture,Func<string,Form,Task> captureDesktop,string path){
  shell=value;host=form;ready=wait;check=assertion;image=capture;desktop=captureDesktop;
  var fixture=Map(json.DeserializeObject(File.ReadAllText(path)));
  check(Convert.ToString(fixture["model"])==Convert.ToString(shell.Work["model"]),"Continuous witnesses match immutable model");
  check(Object.Equals(Map(shell.Work["session"])["raw_full_home"],true),"Continuous baseline is authoritative full Home, not an assumed hash");
  home=Hash;var methods=Items(fixture["methods"]).Select(Map).ToDictionary(m=>Convert.ToString(m["id"]));
  int cycles=0,reuses=0;var sizes=new HashSet<int>();var outcomes=new List<object>();
  foreach(var row in Items(fixture["cycles"]).Select(Map)){
   var m=methods[Convert.ToString(row["method"])];int number=Convert.ToInt32(row["number"]);string id=Convert.ToString(m["id"]);
   check(Hash==home,"Cycle "+number+" starts from previous exact full-label endpoint without reset");
   var affected=new HashSet<int>(Items(m["affected_orbits"]).Select(Convert.ToInt32));
   var preserved=Items(shell.Work["protected"]).Select(Convert.ToInt32).Where(o=>!affected.Contains(o)).ToArray();
   await Command("protect","orbits",preserved);await SelectContext(m,"prepare");
   await Command("draft","phase","macro","recipe",m["setup_recipe"]);await Commit(false);
   check(Hash!=home,"Chosen legal preparation creates a nonidentity working state");
   await SelectContext(m,Convert.ToString(m["goal"]));await Command("bank","id",m["work_bank"]);
   string sheet=Convert.ToString(m["sheet_name"]);
   if(Convert.ToString(row["worksheet"])=="save"){
    await Command("draft","phase","macro","recipe",m["recipe"]);await Command("template-save","name",sheet);
   }else{await LoadSheet(sheet);reuses++;}
   check(json.Serialize(Ordered(Map(Work["draft"])["macro"]))==json.Serialize(Ordered(m["recipe"])),"Saved fixed method retains exact canonical recipe "+id);
   string draft=json.Serialize(Work["draft"]);object current=Work["current"];
   await Command("bank","id",m["alternate_bank"]);await Command("bank","id",m["work_bank"]);
   check(json.Serialize(Work["draft"])==draft&&Object.Equals(Work["current"],current),"Visible bank roundtrip preserves target and fixed steps");
   CheckBindings(m,Convert.ToString(m["goal"]));
   if(number==1||number==4||number==17||number==32){
    await Command("review");foreach(string mode in new[]{"current","operation","after"}){Call("RunCommand","cycles-"+mode);await ready();}
    foreach(var window in Field<Dictionary<string,Form>>("workWindows").Values)window.Hide();
    host.Activate();Call("FocusWorkspace");await desktop("continuous-cycle-"+number,host);
   }
   await Commit(true);check(Hash==home,"Cycle "+number+" restores every one of the 259800 labels");
   check(preserved.All(o=>Items(shell.Work["protected"]).Select(Convert.ToInt32).Contains(o)),"Unaffected existing orbit protection survives cycle "+number);
   if(Convert.ToBoolean(row["checkpoint"])){
    string savedWork=SavedWork();
    await Command("checkpoint","name","continuous-"+number);await Command("undo");check(Hash!=home,"Undo returns to unfinished legal work");
    await Command("redo");check(Hash==home,"Redo restores exact result");await Command("restore","name","continuous-"+number);
    check(Hash==home&&!Convert.ToBoolean(shell.Work["executable"]),"Checkpoint restore retains result without stale permission");
    check(SavedWork()==savedWork,"Checkpoint restore retains canonical inputs, draft, key set and executed ownership");
   }
   cycles++;sizes.Add(Convert.ToInt32(m["sticker_count"]));outcomes.Add(new{cycle=number,method=id,reuse=Convert.ToString(row["worksheet"])!="save",state_hash=Hash});
  }
  check(cycles==32&&reuses==16&&sizes.SetEquals(new[]{1,2,5,20}),"32 continuous mixed cycles include 16 real cross-cycle fixed worksheet reuses");
  File.WriteAllText(Path.Combine(Environment.GetEnvironmentVariable("MAGIC600_NATIVE_OUTPUT"),"continuous-result.json"),json.Serialize(new{scope="Agent-operated native recurring-method stress; not human full solve",cycles=cycles,reuses=reuses,outcomes=outcomes}));
 }
}
