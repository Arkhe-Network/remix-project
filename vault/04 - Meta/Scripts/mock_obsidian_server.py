from http.server import BaseHTTPRequestHandler, HTTPServer
import json

class MockObsidianServer(BaseHTTPRequestHandler):
    def do_PUT(self):
        content_length = int(self.headers['Content-Length'])
        post_data = self.rfile.read(content_length)
        print(f"Received PUT request for {self.path}")
        print(f"Body: {post_data.decode('utf-8')}")
        self.send_response(200)
        self.end_headers()

def run(server_class=HTTPServer, handler_class=MockObsidianServer, port=27123):
    server_address = ('', port)
    httpd = server_class(server_address, handler_class)
    print(f'Starting httpd on port {port}...')
    httpd.serve_forever()

if __name__ == '__main__':
    run()
