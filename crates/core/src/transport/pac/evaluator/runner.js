ObjC.import('Foundation');
ObjC.import('JavaScriptCore');

var PAC_HELPERS = [
  'function dnsResolve(host) { return Object.prototype.hasOwnProperty.call(__dns, host) ? __dns[host] : null; }',
  'function myIpAddress() { return __myIp; }',
  'function isPlainHostName(host) { return host.indexOf(".") < 0; }',
  'function dnsDomainIs(host, domain) { host = host.toLowerCase(); domain = domain.toLowerCase(); return host.length >= domain.length && host.substring(host.length - domain.length) === domain; }',
  'function localHostOrDomainIs(host, hostdom) { return host === hostdom || hostdom.lastIndexOf(host + ".", 0) === 0; }',
  'function isResolvable(host) { return dnsResolve(host) !== null; }',
  'function dnsDomainLevels(host) { return host.split(".").length - 1; }',
  'function convert_addr(ip) { var p = ip.split("."); return ((p[0] << 24) | (p[1] << 16) | (p[2] << 8) | p[3]) >>> 0; }',
  'function isInNet(host, pattern, mask) { var ip = /^\\d+\\.\\d+\\.\\d+\\.\\d+$/.test(host) ? host : dnsResolve(host); if (!ip) { return false; } var m = convert_addr(mask); return ((convert_addr(ip) & m) >>> 0) === ((convert_addr(pattern) & m) >>> 0); }',
  'function shExpMatch(text, pattern) { var re = "^" + pattern.replace(/[.+^${}()|[\\]\\\\]/g, "\\\\$&").replace(/\\*/g, ".*").replace(/\\?/g, ".") + "$"; return new RegExp(re).test(text); }',
  'var __days = ["SUN","MON","TUE","WED","THU","FRI","SAT"];',
  'function weekdayRange(start, end) { var today = new Date().getDay(); var a = __days.indexOf(start); var b = end && end !== "GMT" ? __days.indexOf(end) : a; return a <= b ? today >= a && today <= b : today >= a || today <= b; }',
  'function timeRange(start, end) { var hour = new Date().getHours(); var b = typeof end === "number" ? end : start; return start <= b ? hour >= start && hour <= b : hour >= start || hour <= b; }',
  'function dateRange() { return false; }'
].join('\n');

function readScript(scriptUrl) {
  var url = $.NSURL.URLWithString(scriptUrl);
  if (!url || url.isNil()) { return null; }
  var data = $.NSData.dataWithContentsOfURL(url);
  if (!data || data.isNil()) { return null; }
  var text = $.NSString.alloc.initWithDataEncoding(data, $.NSUTF8StringEncoding);
  if (!text || text.isNil()) {
    text = $.NSString.alloc.initWithDataEncoding(data, $.NSISOLatin1StringEncoding);
  }
  return ObjC.unwrap(text);
}

function resolve(host) {
  var address = $.NSHost.hostWithName(host).address;
  return address && !address.isNil() ? ObjC.unwrap(address) : null;
}

function localAddress() {
  var addresses = ObjC.deepUnwrap($.NSHost.currentHost.addresses) || [];
  for (var i = 0; i < addresses.length; i++) {
    if (/^\d+\.\d+\.\d+\.\d+$/.test(addresses[i]) && addresses[i].indexOf('127.') !== 0) {
      return addresses[i];
    }
  }
  return '127.0.0.1';
}

function run(argv) {
  var source = readScript(argv[0]);
  if (source === null) { return ''; }
  var url = argv[1];
  var host = argv[2];
  var dns = {};
  dns[host] = resolve(host);
  var context = $.JSContext.alloc.init;
  context.evaluateScript('var __dns = ' + JSON.stringify(dns) + '; var __myIp = ' + JSON.stringify(localAddress()) + ';\n' + PAC_HELPERS);
  context.evaluateScript(source);
  var call = 'String(FindProxyForURL(' + JSON.stringify(url) + ', ' + JSON.stringify(host) + '))';
  var result = context.evaluateScript(call);
  return ObjC.unwrap(result.toString);
}
