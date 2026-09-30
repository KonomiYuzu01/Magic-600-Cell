// Agent-operated real native controls and legal E1 fixture; not a human solve.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using System.Windows.Forms;

internal static class WorkIntentNativeChecks {
 static T Field<T>(object value,string name){return (T)value.GetType().GetField(name,BindingFlags.Instance|BindingFlags.NonPublic).GetValue(value);}
 static object Call(object value,string name,params object[] args){return value.GetType().GetMethod(name,BindingFlags.Instance|BindingFlags.NonPublic).Invoke(value,args);}
 static Dictionary<string,object> Map(object value){return (Dictionary<string,object>)value;}
 internal static async Task Run(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check,Action<string,Form> image){
  await ready();var initial=Map(shell.Work["workspace"]);string hash=Convert.ToString(shell.Work["hash"]);int identity=Convert.ToInt32(initial["current"]);
  var picker=Field<ComboBox>(shell,"workGoal");
  foreach(string blocked in new[]{"busy","stopPending"}){
   var flag=typeof(ExperimentShell).GetField(blocked,BindingFlags.Instance|BindingFlags.NonPublic);string actualGoal=Convert.ToString(initial["goal"]);
   flag.SetValue(shell,true);Call(shell,"RefreshCommandAvailability");
   try{
    picker.SelectedItem=picker.Items.Cast<object>().Single(item=>Field<string>(item,"Id")=="orient");
    typeof(ComboBox).GetMethod("OnSelectionChangeCommitted",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(picker,new object[]{EventArgs.Empty});
    check(Field<string>(picker.SelectedItem,"Id")==actualGoal&&Convert.ToString(Map(shell.Work["workspace"])["goal"])==actualGoal,"Rejected goal while "+blocked+" cannot leave a false selected caption");
    check(!picker.Enabled,"Goal picker is disabled while "+blocked);
   }finally{flag.SetValue(shell,false);Call(shell,"RefreshCommandAvailability");}
  }
  foreach(string goal in new[]{"prepare","insert","place","orient","finish-buffer","block","endgame"}){
   picker.SelectedItem=picker.Items.Cast<object>().Single(item=>Field<string>(item,"Id")==goal);
   typeof(ComboBox).GetMethod("OnSelectionChangeCommitted",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(picker,new object[]{EventArgs.Empty});await ready();
   check(Convert.ToString(Map(shell.Work["workspace"])["goal"])==goal,"Existing Solve selector binds explicit goal "+goal);
  }
  check(await shell.Send(LocalApi.D("action","bank","id","Operation")),"Open the visible Operation key set");await ready();
  var input=Field<ExperimentInput>(shell,"input");host.Activate();Call(shell,"FocusWorkspace");
  input.HandleKeyDown("Digit9",false,false,false,false,false,false);input.HandleKeyUp("Digit9");await ready();
  check(Convert.ToString(Map(shell.Work["workspace"])["goal"])=="orient","Single 9 key selects Orient piece through the real input route");
  var recipe=new object[]{LocalApi.D("kind","star","orbit",33,"node",0,"sign",-1)};
  check(await shell.Send(LocalApi.D("action","draft","phase","macro","recipe",recipe)),"Supply one explicitly chosen inverse star");await ready();
  check(await shell.Send(LocalApi.D("action","review")),"Review missing orientation requirement without inventing a goal");await ready();
  var result=Map(Map(shell.Work["review"])["goal_result"]);
  check(Convert.ToString(result["status"])=="MissingInput","Orient reports missing exact requirement while preserving the operation");
  check(Convert.ToString(Map(shell.Work["review"])["status"])=="Ready","Goal input and legal protection-compliant execution remain separate");
  host.Activate();Call(shell,"FocusWorkspace");input.HandleKeyDown("Minus",false,false,false,false,false,false);input.HandleKeyUp("Minus");await ready();
  var requirement=Map(Map(shell.Work["workspace"])["target_requirement"]);
  check(Convert.ToInt32(requirement["identity"])==identity&&Convert.ToString(Map(requirement["provenance"])["kind"])=="home","Single Home-goal key records the explicitly chosen canonical requirement");
  check(Convert.ToString(shell.Work["hash"])==hash,"Changing goals and their references never turns the puzzle");
  check(await shell.Send(LocalApi.D("action","review")),"Recheck explicit orientation requirement on the same full operation");await ready();
  result=Map(Map(shell.Work["review"])["goal_result"]);
  check(Convert.ToString(result["kind"])=="OrientPiece"&&Convert.ToString(result["status"])=="Met"&&Convert.ToBoolean(result["conditional_after"]),"Known E1 operation conditionally meets the declared exact goal");
  Call(shell,"RunCommand","solve-prepare");await ready();
  var windows=Field<Dictionary<string,Form>>(shell,"workWindows");var solve=windows["solve"];
  foreach(var role in Field<Dictionary<string,Button>>(shell,"solveRoles")){var button=role.Value;var measured=TextRenderer.MeasureText(button.Text,button.Font,new Size(Math.Max(1,button.Width-button.Padding.Horizontal-6),Int32.MaxValue),TextFormatFlags.WordBreak);Console.WriteLine("Role text "+role.Key+": "+button.Text.Replace("\n","\\n")+"; bounds="+button.Bounds+" client="+button.ClientSize+" padding="+button.Padding+" font="+button.Font+" measured="+measured);}
  foreach(var role in Field<Dictionary<string,Button>>(shell,"solveRoles")){var button=role.Value;int needed=button.GetPreferredSize(new Size(button.Width,Int32.MaxValue)).Height;check(button.Height>=needed,"Role "+role.Key+" fits its native button text layout: needs "+needed+", available "+button.Height);}
  image("solve-work-intents",solve);
  Call(shell,"RunCommand","keyboard");await ready();image("keyboard-work-intents",windows["keyboard"]);
  check(await shell.Send(LocalApi.D("action","preview","review_id",Map(shell.Work["review"])["id"])),"Stage only the user-chosen reviewed operation");await ready();
  check(await shell.Send(LocalApi.D("action","commit")),"Explicitly execute through the existing Session transaction");await ready();
  check(Convert.ToString(shell.Work["hash"])!=hash,"Commit adopts the exact changed mechanical state");
  check(await shell.Send(LocalApi.D("action","undo")),"Undo the explicit intent operation");await ready();
  check(Convert.ToString(shell.Work["hash"])==hash,"Undo restores all labels exactly");
 }
}
