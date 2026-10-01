// Deterministic delayed stop acknowledgement, using the real native admission boundary.
// The test-only loopback responder delays LocalApi Stop; busy true/false models
// the earlier request completing before acknowledgement. No model/state is faked.
using System;
using System.Collections.Generic;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Reflection;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;

internal static class StopAcknowledgementNativeChecks {
 static T Field<T>(ExperimentShell shell,string name){return (T)typeof(ExperimentShell).GetField(name,BindingFlags.Instance|BindingFlags.NonPublic).GetValue(shell);}
 static string Context(ExperimentShell shell){return new JavaScriptSerializer{MaxJsonLength=64000000}.Serialize(new object[]{shell.Work["hash"],shell.Work["workspace"],shell.Work["pending"]});}
 static object Call(ExperimentShell shell,string name,params object[] args){return typeof(ExperimentShell).GetMethod(name,BindingFlags.Instance|BindingFlags.NonPublic).Invoke(shell,args);}
 static async Task LayoutSettled(ExperimentShell shell){
  var deadline=DateTime.UtcNow.AddSeconds(15);while(shell.WindowLayoutPending){if(shell.WindowLayoutFailure!=null)throw new InvalidOperationException(shell.WindowLayoutFailure);if(DateTime.UtcNow>deadline)throw new TimeoutException("Stop fixture window layout did not settle");await Task.Delay(25);}
 }
 internal static async Task Run(ExperimentShell shell,Func<Task> ready,Action<bool,string> check){
  await ready();await LayoutSettled(shell);
  check(await shell.Send(LocalApi.D("action","inspect-phase")),"Stop fixture captures fresh authoritative work before temporary keyboard context");await ready();
  var workspace=(Dictionary<string,object>)shell.Work["workspace"];string bank=Convert.ToString(workspace["bank"]),previousBank=Convert.ToString(workspace["previous_bank"]),context=Context(shell);
  var api=Field<LocalApi>(shell,"api");byte[] labels=await Task.Factory.StartNew(()=>api.Bytes("labels"));
  var oldKeyboard=Field<Form>(shell,"keyboardWindow");bool visible=oldKeyboard!=null&&!oldKeyboard.IsDisposed&&oldKeyboard.Visible;
  var active=Form.ActiveForm;bool restoring=Field<bool>(shell,"restoringWindows");Exception failure=null;
  try{
   if(bank!="33-A"){check(previousBank!=bank,"Stop fixture can restore the distinct previous key set through normal bank commands");check(await shell.Send(LocalApi.D("action","bank","id","33-A")),"Stop fixture selects real 33-A Grip bindings");await ready();}
   typeof(ExperimentShell).GetField("restoringWindows",BindingFlags.Instance|BindingFlags.NonPublic).SetValue(shell,true);
   try{Call(shell,"OpenFloatingKeyboard",false);}finally{typeof(ExperimentShell).GetField("restoringWindows",BindingFlags.Instance|BindingFlags.NonPublic).SetValue(shell,restoring);}
   var keyboard=Field<Form>(shell,"keyboardWindow");keyboard.Activate();Field<FlowLayoutPanel>(shell,"keyboard").Focus();Field<ExperimentInput>(shell,"input").Reset("Stop fixture starts with released input.");
   await RunBarrier(shell,ready,check);
  }catch(Exception error){failure=error;}
  // .NET Framework compiler: async restoration is deliberately outside finally.
  try{
   await ready();
   if(bank!="33-A"){check(await shell.Send(LocalApi.D("action","bank","id",previousBank)),"Stop fixture restores the former previous key set");await ready();check(await shell.Send(LocalApi.D("action","bank","id",bank)),"Stop fixture restores the original active key set");await ready();}
   var keyboard=Field<Form>(shell,"keyboardWindow");typeof(ExperimentShell).GetField("restoringWindows",BindingFlags.Instance|BindingFlags.NonPublic).SetValue(shell,true);
   try{if(!visible&&keyboard!=null&&!keyboard.IsDisposed)keyboard.Hide();}finally{typeof(ExperimentShell).GetField("restoringWindows",BindingFlags.Instance|BindingFlags.NonPublic).SetValue(shell,restoring);}
   if(active!=null&&!active.IsDisposed&&active.Visible)active.Activate();
   byte[] restored=await Task.Factory.StartNew(()=>api.Bytes("labels"));check(labels.SequenceEqual(restored)&&labels.Length==259800*4,"Stop keyboard fixture preserves every committed label");
   check(Context(shell)==context,"Stop keyboard fixture restores bank, previous bank, Current/Next, draft, preferences and pending work");
  }catch(Exception cleanup){if(failure!=null)throw new AggregateException(failure,cleanup);throw;}
  if(failure!=null)throw failure;
 }
 static async Task RunBarrier(ExperimentShell shell,Func<Task> ready,Action<bool,string> check){
  await ready();var input=Field<ExperimentInput>(shell,"input");var state=(ExperimentInputState)Call(shell,"InputState");
  var physical=Field<Dictionary<Button,string>>(shell,"physicalKeyboardKeys");var grip=state.GripKeys.First(pair=>pair.Value>=1&&pair.Value<=600&&!state.CommandKeys.ContainsKey(pair.Key));
  var gripButton=physical.Single(pair=>pair.Value==grip.Key).Key;var bankKey=state.CommandKeys.First(pair=>pair.Value=="bank").Key;var bankButton=physical.Single(pair=>pair.Value==bankKey).Key;
  bool initialGripReady=state.BankId=="33-A"&&Field<Form>(shell,"keyboardWindow").Visible&&gripButton.Enabled&&!input.InverseShiftHeld&&!input.ActiveGripCell.HasValue;
  if(!initialGripReady)NativeDiagnostics.Write("Stop fixture initial Grip mismatch: "+new JavaScriptSerializer().Serialize(new{bank=state.BankId,windowVisible=Field<Form>(shell,"keyboardWindow").Visible,windowEnabled=Field<Form>(shell,"keyboardWindow").Enabled,keyEnabled=gripButton.Enabled,shift=input.InverseShiftHeld,activeGrip=input.ActiveGripCell,ready=shell.IsReady,inputState=state,feedback=Field<Label>(shell,"inputFeedback").Text}));
  check(initialGripReady,"Stop fixture starts with an enabled, assigned real Grip and released modifiers");
  string context=Context(shell);var original=Field<LocalApi>(shell,"api");var apiField=typeof(ExperimentShell).GetField("api",BindingFlags.Instance|BindingFlags.NonPublic);
  var listener=new TcpListener(IPAddress.Loopback,0);listener.Start();int port=((IPEndPoint)listener.LocalEndpoint).Port;
  using(var received=new ManualResetEvent(false))using(var release=new ManualResetEvent(false)){
   var response=Task.Factory.StartNew(delegate{
    using(var client=listener.AcceptTcpClient())using(var stream=client.GetStream()){
     client.ReceiveTimeout=15000;var header=new StringBuilder();while(!header.ToString().EndsWith("\r\n\r\n",StringComparison.Ordinal)){int value=stream.ReadByte();if(value<0)throw new InvalidOperationException("Stop connection ended before headers");header.Append((char)value);if(header.Length>16000)throw new InvalidOperationException("Unexpected fixture request header");}
     string request=header.ToString();if(!request.StartsWith("POST /api/stop-job ",StringComparison.Ordinal))throw new InvalidOperationException("Fixture accepts only stop-job");
     var lengthLine=request.Split(new[]{"\r\n"},StringSplitOptions.None).FirstOrDefault(line=>line.StartsWith("Content-Length:",StringComparison.OrdinalIgnoreCase));int remaining=lengthLine==null?0:Int32.Parse(lengthLine.Substring(lengthLine.IndexOf(':')+1).Trim());while(remaining-->0)if(stream.ReadByte()<0)throw new InvalidOperationException("Incomplete stop body");received.Set();
     if(!release.WaitOne(15000))throw new TimeoutException("Stop acknowledgement was not released");
     byte[] body=Encoding.UTF8.GetBytes("{\"cancel_requested\":true}");byte[] head=Encoding.ASCII.GetBytes("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: "+body.Length+"\r\nConnection: close\r\n\r\n");stream.Write(head,0,head.Length);stream.Write(body,0,body.Length);stream.Flush();
    }
   });
   try{
    typeof(ExperimentShell).GetField("busy",BindingFlags.Instance|BindingFlags.NonPublic).SetValue(shell,true);
    apiField.SetValue(shell,new LocalApi("http://127.0.0.1:"+port,"isolated-stop-fixture"));typeof(ExperimentShell).GetMethod("StopAnalysis",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(shell,null);
    var deadline=DateTime.UtcNow.AddSeconds(15);while(!received.WaitOne(0)){if(response.IsFaulted)await response;if(DateTime.UtcNow>deadline)throw new TimeoutException("Stop request did not reach fixture responder");await Task.Delay(15);}
    apiField.SetValue(shell,original);typeof(ExperimentShell).GetField("busy",BindingFlags.Instance|BindingFlags.NonPublic).SetValue(shell,false);
    Call(shell,"RefreshCommandAvailability");
    check(!Field<bool>(shell,"busy")&&!shell.IsReady,"Completed work stays unavailable while its stop acknowledgement is outstanding");
    check(!gripButton.Enabled&&bankButton.Enabled,"Outstanding Stop disables real Grip keycaps while keeping the explicit bank tool available");
    input.HandleKeyDown(grip.Key,false,false,false,false,false);input.HandleKeyUp(grip.Key);
    check(!input.ActiveGripCell.HasValue&&!input.Pending,"Direct Grip input cannot acquire a cap or dispatch while Stop is outstanding");
    bool accepted=await shell.Send(LocalApi.D("action","inspect-phase"));check(!accepted,"A subsequent command is rejected while delayed Stop is unacknowledged");
    check(Context(shell)==context,"Rejected next command leaves full labels, Current/Next, draft and pending work unchanged");
    release.Set();await response;await ready();check(shell.IsReady,"Successful Stop acknowledgement reopens the explicit command boundary");
    check(gripButton.Enabled,"The same real Grip key becomes available after acknowledged Stop");
    check(await shell.Send(LocalApi.D("action","inspect-phase")),"A fresh explicit command is accepted after the stop barrier clears");await ready();
    check(Context(shell)==context,"Post-ack read-only inspection retains exact work and does not replay the rejected command");
   }finally{apiField.SetValue(shell,original);typeof(ExperimentShell).GetField("busy",BindingFlags.Instance|BindingFlags.NonPublic).SetValue(shell,false);release.Set();listener.Stop();}
  }
 }
}
