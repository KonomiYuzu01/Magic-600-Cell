// Source/fixture checks only; no WinForms, DirectX, engine or user session.
using System;
using System.Collections.Generic;
using System.IO;

internal static class ExperimentSendOutcomeRegression {
 static void Assert(bool value,string message){if(!value)throw new InvalidOperationException(message);}
 static Dictionary<string,object> D(params object[] pairs){var result=new Dictionary<string,object>();for(int i=0;i<pairs.Length;i+=2)result.Add((string)pairs[i],pairs[i+1]);return result;}
 static void UnrecoverableCommit(){
  var receipt=D("committed",true);var reply=D("result",receipt,"requires_refresh",true);var fault=new IOException("Display refresh failed.");var recoveryFault=new IOException("Recovery failed.");var calls=new List<string>();Exception reported=null;int refreshes=0;
  var outcome=ExperimentSendOutcome.Worker<object>(()=>{calls.Add("post");return reply;},()=>{calls.Add("refresh");throw ++refreshes==1?fault:recoveryFault;},(value,useBase)=>{throw new InvalidOperationException("No snapshot should be read.");},"commit",error=>{calls.Add("recoveryFailed");reported=error;});
  Assert(Object.ReferenceEquals(outcome.Reply,reply)&&Object.ReferenceEquals(outcome.Receipt,receipt)&&outcome.Next==null&&Object.ReferenceEquals(outcome.Fault,fault),"A committed receipt or primary fault was lost after both refreshes failed.");
  Assert(String.Join(",",calls)=="post,refresh,refresh,recoveryFailed"&&Object.ReferenceEquals(reported,recoveryFault),"Recovery order or failure reporting changed.");
  var finalized=ExperimentSendOutcome.Finalize(outcome.Receipt,false,receipt,outcome.Fault,false,true);
  Assert(Object.ReferenceEquals(finalized.LastResult,receipt)&&finalized.Accepted&&!finalized.LogRejection,"An unrecoverable committed command was rejected.");
  Assert(finalized.ReceiptWarning=="Operation was committed. Display refresh failed. Input remains disabled; reopen this session to recover its native display. Do not repeat the operation.","The committed warning must preserve its fault and prevent a repeat.");
  Console.WriteLine("PASS: committed command keeps its receipt when refresh and recovery fail");
 }
 static void RecoveredCommit(){
  var receipt=D("committed",true);var primary=D("result",receipt,"requires_refresh",true);var recoveryResult=D("recovery",true);var recovered=D("result",recoveryResult);var snapshot=new object();var fault=new IOException("Display refresh failed.");int posts=0,refreshes=0,reads=0;
  var outcome=ExperimentSendOutcome.Worker<object>(()=>{posts++;return primary;},()=>{if(++refreshes==1)throw fault;return recovered;},(value,useBase)=>{reads++;Assert(!useBase&&Object.ReferenceEquals(value,recovered),"Recovery must read the fresh reply without a base.");return snapshot;},"twist",error=>{throw new InvalidOperationException("Recovery succeeded.");});
  Assert(posts==1&&refreshes==2&&reads==1&&Object.ReferenceEquals(outcome.Next,snapshot)&&Object.ReferenceEquals(outcome.Fault,fault),"A recovery must not repost or clear the original fault.");
  Assert(Object.ReferenceEquals(outcome.Receipt,receipt)&&Object.ReferenceEquals(outcome.Reply,recovered)&&Object.ReferenceEquals(recovered["result"],recoveryResult),"Recovery must keep the receipt separately from the recovered reply.");
  var finalized=ExperimentSendOutcome.Finalize(outcome.Receipt,false,recoveryResult,outcome.Fault,true,true);
  Assert(Object.ReferenceEquals(finalized.LastResult,receipt)&&finalized.Accepted&&!finalized.LogRejection,"A recovered committed command was rejected.");
  Assert(finalized.ReceiptWarning=="Operation was committed. Display refresh failed. Current state was refreshed. Do not repeat the operation.","The recovered warning must report refresh and prevent a repeat.");
  Console.WriteLine("PASS: recovery adopts the snapshot and preserves the committed receipt");
 }
 static void SuccessfulReplies(){
  foreach(bool committed in new[]{false,true})foreach(bool requiresRefresh in new[]{false,true}){
   var primaryResult=D("committed",committed,"warning","Engine warning.");var reply=D("result",primaryResult,"requires_refresh",requiresRefresh);var fresh=D("result",D("stale",true));var snapshot=new object();int refreshes=0,reads=0;
   var outcome=ExperimentSendOutcome.Worker<object>(()=>reply,()=>{refreshes++;return fresh;},(value,useBase)=>{reads++;Assert(useBase==!requiresRefresh,"Successful reads must use the base only without a refresh.");Assert(Object.ReferenceEquals(value,requiresRefresh?fresh:reply)&&Object.ReferenceEquals(value["result"],primaryResult),"Refresh must carry the primary result into the fresh reply.");return snapshot;},"commit",error=>{throw new InvalidOperationException("No recovery failure expected.");});
   Assert(refreshes==(requiresRefresh?1:0)&&reads==1&&outcome.Fault==null&&Object.ReferenceEquals(outcome.Next,snapshot),"Successful worker flow changed.");
   Assert(committed?Object.ReferenceEquals(outcome.Receipt,primaryResult):outcome.Receipt==null,"Only a committed success has an operation receipt.");
   var finalized=ExperimentSendOutcome.Finalize(outcome.Receipt,false,primaryResult,null,true,true);
   Assert(Object.ReferenceEquals(finalized.LastResult,primaryResult)&&finalized.Accepted&&!finalized.LogRejection&&finalized.ReceiptWarning==null,"A successful result or warning state was lost.");
   var incomplete=ExperimentSendOutcome.Finalize(outcome.Receipt,false,primaryResult,null,true,false);
   Assert(incomplete.Accepted==committed,"An incomplete phase must reject an uncommitted success and retain a committed receipt.");
   var disconnected=ExperimentSendOutcome.Finalize(outcome.Receipt,false,primaryResult,null,false,true);
   Assert(disconnected.Accepted==committed&&!disconnected.LogRejection,"Display adoption failure must preserve only receipt acceptance.");
   Assert(disconnected.ReceiptWarning==(committed?"Operation was committed. Native display adoption failed. Input remains disabled; reopen this session to recover its native display. Do not repeat the operation.":null),"Display adoption failure must warn only for a receipt.");
   Assert((string)primaryResult["warning"]=="Engine warning.","Finalization must not mutate the engine receipt.");
  }
  Console.WriteLine("PASS: committed and uncommitted successes preserve refresh, base and phase rules");
 }
 static void FailedPost(){
  var fault=new IOException("Post failed.");var fresh=D("result",D("recovery",true));var snapshot=new object();int posts=0,refreshes=0;
  var outcome=ExperimentSendOutcome.Worker<object>(()=>{posts++;throw fault;},()=>{refreshes++;return fresh;},(value,useBase)=>{Assert(!useBase,"Post failure recovery must read without a base.");return snapshot;},"commit",error=>{throw new InvalidOperationException("Recovery succeeded.");});
  Assert(posts==1&&refreshes==1&&outcome.Receipt==null&&Object.ReferenceEquals(outcome.Fault,fault)&&Object.ReferenceEquals(outcome.Next,snapshot),"Post failure must recover once without inventing a receipt.");
  var finalized=ExperimentSendOutcome.Finalize(outcome.Receipt,false,(Dictionary<string,object>)fresh["result"],outcome.Fault,true,true);
  Assert(finalized.LastResult==null&&!finalized.Accepted&&finalized.LogRejection&&finalized.ReceiptWarning==null,"A failed post was accepted because recovery succeeded.");
  Console.WriteLine("PASS: an uncommitted failed post remains rejected after recovery");
 }
 static void AppliedImport(){
  var receipt=D("import_applied",true);var primary=D("result",receipt,"requires_refresh",true);var fault=new IOException("Import refresh failed.");int refreshes=0,recoveryFailures=0;
  var outcome=ExperimentSendOutcome.Worker<object>(()=>primary,()=>{refreshes++;throw fault;},(value,useBase)=>{throw new InvalidOperationException("No snapshot should be read.");},"session-log-apply",error=>{recoveryFailures++;});
  Assert(Object.ReferenceEquals(outcome.Receipt,receipt)&&refreshes==2&&recoveryFailures==1,"An applied import must retain its receipt without a committed flag.");
  var disconnected=ExperimentSendOutcome.Finalize(outcome.Receipt,true,receipt,outcome.Fault,false,true);
  Assert(Object.ReferenceEquals(disconnected.LastResult,receipt)&&disconnected.Accepted&&!disconnected.LogRejection,"An applied import was rejected.");
  Assert(disconnected.ReceiptWarning=="Log import was accepted. Import refresh failed. Input remains disabled; reopen this session to recover its native display. Do not repeat the import.","Disconnected import wording changed.");
  var connected=ExperimentSendOutcome.Finalize(outcome.Receipt,true,receipt,outcome.Fault,true,true);
  Assert(connected.Accepted&&connected.ReceiptWarning=="Log import was accepted. Import refresh failed. Current state was refreshed.","Recovered import wording changed.");
  var adoption=ExperimentSendOutcome.Finalize(outcome.Receipt,true,receipt,null,false,true);
  Assert(adoption.Accepted&&adoption.ReceiptWarning=="Log import was accepted. Native display adoption failed. Input remains disabled; reopen this session to recover its native display. Do not repeat the import.","Import adoption-failure wording changed.");
  Console.WriteLine("PASS: applied import stays accepted with the existing exact warning wording");
 }
 static void InvalidBase(){
  var receipt=D("committed",true);var reply=D("result",receipt);var fresh=D();var snapshot=new object();var calls=new List<string>();int refreshes=0;
  var outcome=ExperimentSendOutcome.Worker<object>(()=>{calls.Add("post");return reply;},()=>{calls.Add("refresh");refreshes++;return fresh;},(value,useBase)=>{calls.Add(useBase?"base":"full");if(useBase)throw new InvalidDataException("Stale base.");Assert(Object.ReferenceEquals(value,fresh),"The fallback must read the fresh snapshot.");return snapshot;},"commit",error=>{throw new InvalidOperationException("Fallback succeeded.");});
  Assert(refreshes==1&&String.Join(",",calls)=="post,base,refresh,full"&&outcome.Fault==null&&Object.ReferenceEquals(outcome.Next,snapshot),"An invalid base must refresh exactly once without an error recovery.");
  Assert(Object.ReferenceEquals(outcome.Reply,fresh)&&Object.ReferenceEquals(fresh["result"],receipt)&&Object.ReferenceEquals(outcome.Receipt,receipt),"Base fallback must carry the primary result and preserve the receipt.");
  Console.WriteLine("PASS: InvalidDataException retries once without a base and carries the result");
 }
 static int Main(){try{UnrecoverableCommit();RecoveredCommit();SuccessfulReplies();FailedPost();AppliedImport();InvalidBase();return 0;}catch(Exception error){Console.Error.WriteLine(error);return 1;}}
}
