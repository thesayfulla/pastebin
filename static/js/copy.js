function copyCode() {
  var codeElement = document.getElementById('code');
  var range = document.createRange();
  range.selectNode(codeElement);
  window.getSelection().removeAllRanges();
  window.getSelection().addRange(range);
  document.execCommand('copy');
  window.getSelection().removeAllRanges();
  alert('Code copied to clipboard!');
}
