// Pascal — TArray helpers, sorted ascending.

program TaskArray;

{$APPTYPE CONSOLE}

uses
  SysUtils, Generics.Collections;

type
  TTask = record
    Title: string;
    Priority: Integer;
  end;

var
  backlog: TArray<TTask>;

function FindByTitle(const arr: TArray<TTask>; const title: string): Integer;
var
  i: Integer;
begin
  Result := -1;
  for i := 0 to High(arr) do
    if arr[i].Title = title then
      Exit(i);
end;

procedure SortAsc(var arr: TArray<TTask>);
begin
  TArray.Sort<TTask>(arr,
    TComparer<TTask>.Construct(
      function(const L, R: TTask): Integer
      begin
        Result := L.Priority - R.Priority;
      end));
end;

begin
  SetLength(backlog, 3);
  backlog[0].Title := 'Write tests';     backlog[0].Priority := 2;
  backlog[1].Title := 'Fix login bug';  backlog[1].Priority := 5;
  backlog[2].Title := 'Refactor';       backlog[2].Priority := 3;

  SortAsc(backlog);
  Writeln('idx of "Fix login bug": ', FindByTitle(backlog, 'Fix login bug'));
end.