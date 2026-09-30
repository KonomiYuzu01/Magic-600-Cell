// Agent-operated legal fixture and real native controls, not a human solve.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;
internal static class CurrentScoreNativeChecks {
 static T Field<T>(ExperimentShell shell,string name){return (T)typeof(ExperimentShell).GetField(name,BindingFlags.Instance|BindingFlags.NonPublic).GetValue(shell);}
 static void Call(ExperimentShell shell,string name,params object[] args){typeof(ExperimentShell).GetMethod(name,BindingFlags.Instance|BindingFlags.NonPublic).Invoke(shell,args);}
 static Dictionary<string,object> Map(object value){return value as Dictionary<string,object>;}
 static object[] Items(object value){return value as object[]??new object[0];}
 static string Stable(ExperimentShell shell){var w=Map(shell.Work["workspace"]);return new JavaScriptSerializer().Serialize(new object[]{shell.Work["hash"],w["current"],w["next"],w["roles"],w["draft"],w["bank"],shell.Work["protected"]});}
 internal static async Task Run(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check,Action<string,Form> image,Func<string,Form,Task> desktop){
  check(await shell.Send(LocalApi.D("action","insert-macro","id","o33-n0-inverse","phase","macro")),"Explicitly add the existing E1 inverse star");await ready();
  check(await shell.Send(LocalApi.D("action","bank","id","Operation")),"Open visible Operation key set");await ready();
  check(await shell.Send(LocalApi.D("action","review")),"Review the chosen complete operation");await ready();
  var review=Map(shell.Work["review"]);var result=Map(review["recommendation"]);check(Convert.ToString(result["category"])=="Direct"&&Convert.ToDouble(Map(result["terms"])["F"])==1,"Native receives measured focused-goal benefit from the same review");
  string stable=Stable(shell);Call(shell,"RunCommand","solve-protection");await ready();
  var solve=Field<Dictionary<string,Form>>(shell,"workWindows")["solve"];solve.Activate();await Task.Delay(200);
  check(Field<Label>(shell,"recommendationCaption").Text.StartsWith("Direct benefit"),"Existing Solve protection page presents current benefit separately from protection");
  var buttons=Field<List<Button>>(shell,"recommendationReasons");check(buttons.Count(b=>b.Visible)==Items(result["reasons"]).Length,"Every returned reason has one existing-workflow inspection entry");
  foreach(var button in buttons.Where(b=>b.Visible))check(button.Height>=button.GetPreferredSize(new Size(button.Width,Int32.MaxValue)).Height,"Reason text fits its actual native button layout");
  check(buttons[0].Text.EndsWith("[")&&buttons[1].Text.EndsWith("]")&&buttons[2].Text.EndsWith(";"),"Reason hints match the existing onscreen key symbols");
  await desktop("recommendation-before-gdi",solve);image("recommendation-solve",solve);await desktop("recommendation-after-gdi",solve);
  var originalSize=solve.Size;solve.Size=solve.MinimumSize;await Task.Delay(150);foreach(var button in buttons.Where(b=>b.Visible))check(button.Height>=button.GetPreferredSize(new Size(button.Width,Int32.MaxValue)).Height,"Compact Solve preserves readable reason labels");image("recommendation-solve-compact",solve);check(Stable(shell)==stable,"Solve resizing preserves work state");solve.Size=originalSize;await Task.Delay(150);
  host.Activate();Call(shell,"FocusWorkspace");var input=Field<ExperimentInput>(shell,"input");
  input.HandleKeyDown("Backslash",false,false,false,false,false,false);input.HandleKeyUp("Backslash");await ready();check(Convert.ToString(Map(shell.Work["workspace"])["bank"])=="Functions","Dedicated Functions key remains available in Operation set");
  input.HandleKeyDown("Backslash",false,false,false,false,false,false);input.HandleKeyUp("Backslash");await ready();check(Stable(shell)==stable,"Functions return preserves Operation set and all work context");
  input.HandleKeyDown("BracketLeft",false,false,false,false,false,false);input.HandleKeyUp("BracketLeft");await ready();
  var hub=Field<ExperimentHub>(shell,"hub");check(hub.CycleProjection!=null&&Convert.ToString(hub.CycleProjection["mode"])=="Operation"&&Convert.ToString(hub.CycleProjection["scope"])=="complete","Single bracket key opens complete Operation evidence");
  check(hub.ReviewEvidenceVisibleCount>0,"Measured reason outlines canonical identity tokens in the existing graph");check(Stable(shell)==stable,"Reason inspection preserves complete state, Current, locked Next, roles, draft, key set and protection");check(!solve.Visible,"Reason inspection uncovers the graph by closing its supporting Solve window");await desktop("recommendation-evidence",host);image("recommendation-evidence",host);
  Call(shell,"RunCommand","operation-focus");await ready();check(solve.Visible&&shell.ActiveSolvePage=="protection"&&Stable(shell)==stable,"Return to Solve restores the same page, result and work context");
  check(await shell.Send(LocalApi.D("action","goal","goal","prepare")),"Change explicit goal without executing");await ready();
  check(Convert.ToString(Map(Map(shell.Work["review"])["recommendation"])["status"])=="Stale","A new work goal withdraws the old recommendation");check(hub.ReviewEvidenceVisibleCount==0,"Stale review removes old evidence outlines");
  Call(shell,"RunCommand","review-reason-1");check(Field<Label>(shell,"feedback").Text.Contains("out of date"),"Old reason cannot silently inspect another current result");
  check(await shell.Send(LocalApi.D("action","review")),"Refresh the explicit complete operation");await ready();
  check(await shell.Send(LocalApi.D("action","preview","review_id",Map(shell.Work["review"])["id"])),"Stage through the existing guarded preview boundary");await ready();
  string before=Convert.ToString(shell.Work["hash"]);check(await shell.Send(LocalApi.D("action","commit")),"Explicitly commit the chosen operation");await ready();check(Convert.ToString(shell.Work["hash"])!=before,"Commit changes actual labels");check(await shell.Send(LocalApi.D("action","undo")),"Undo the committed operation");await ready();check(Convert.ToString(shell.Work["hash"])==before,"Undo restores all labels exactly");
  check(await shell.Send(LocalApi.D("action","draft","phase","macro","recipe",new[]{LocalApi.D("kind","word","moves",new[]{1,-1})})),"Explicit legal forward/inverse operation has zero net action");await ready();
  check(await shell.Send(LocalApi.D("action","protect","orbits",Enumerable.Range(0,35).ToArray())),"Fixture explicitly protects all moving orbits");await ready();
  check(await shell.Send(LocalApi.D("action","prefix","strict",true)),"User explicitly chooses Strict protection");await ready();
  check(await shell.Send(LocalApi.D("action","review")),"Check temporary action through the existing full-prefix review");await ready();
  review=Map(shell.Work["review"]);check(Convert.ToString(review["status"])=="Conflict"&&Convert.ToString(review["post_hash"])==Convert.ToString(shell.Work["hash"]),"Strict rejects genuine intermediate damage despite exact net preservation");
  stable=Stable(shell);Call(shell,"RunCommand","review-reason-1");await ready();
  check(Convert.ToString(Map(hub.CycleProjection["selected"])["kind"])=="unchanged","Temporary conflict inspects its actual fixed position without inventing a net cycle");
  check(hub.ReviewEvidenceVisibleCount>0&&Stable(shell)==stable,"Temporary evidence highlights canonical identities without changing state or work");await desktop("recommendation-prefix-evidence",host);image("recommendation-prefix-evidence",host);
 }
}
