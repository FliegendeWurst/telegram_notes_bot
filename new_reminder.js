// Create as JS Backend note with these attributes:
// ~targetTemplate=@reminder template
// #customRequestHandler=new_reminder
// Create a new note (reminder template) with these promoted attributes:
// todoDate: date
// todoTime: text
// doneDate: date
// reminder = true

Date.prototype.addHours = function(h) {
    this.setTime(this.getTime() + (h*60*60*1000));
    return this;
}

const {req, res} = api;

const time = new Date(req.body["time"]);
api.log(time);
const task = req.body["task"];

var hour = time.getHours();
var minute = time.getMinutes();
var second = time.getSeconds();

if (hour < 10) {
  hour = '0' + hour;
}
if (minute < 10) {
  minute = '0' + minute;
}
if (second < 10) {
  second = '0' + second;
}

const year = time.getFullYear();
var month = time.getMonth() + 1;
var day = time.getDate();

if (month < 10) {
  month = '0' + month;
}
if (day < 10) {
  day = '0' + day;
}

const todayDateStr = year + "-" + month + "-" + day;
const todayNote = await api.getDateNote(todayDateStr);

const targetTemplate = await api.currentNote.getAttributeValue('relation', 'targetTemplate');
const resp = await api.createNote(
    todayNote.noteId,
    task,
    '',
    {
        "attributes": [
            {"type": "relation", "name": "template", "value": targetTemplate},
            {"type": "label", "name": "todoDate", "value": todayDateStr},
            {"type": "label", "name": "todoTime", "value": hour + ":" + minute + ":" + second}
    ]}
);

res.sendStatus(200);
