#!/usr/bin/env ruby

require 'json'
require 'net/http'
require 'socket'
require 'base64'

def request(method, path, body=nil)
  body_json = JSON.generate(body) if body
  uri = URI("http://localhost:50080#{path}")
  response = Net::HTTP.start(uri.host, uri.port) do |http|
    request = Net::HTTP.const_get(method.capitalize).new(uri)
    request.body = body_json
    request['Content-Type'] = 'application/json'
    http.request(request)
  end

  if response.code.to_i >= 400
    raise "Request failed with status #{response.code}: #{response.body}"
  end

  if response.header['Content-Type'] == 'application/octet-stream'
    return response.body
  end

  JSON.parse(response.body)
end

def run_with_derrick(options)
  command = "cargo run -- #{options.join(' ')}"
  pid = Process.spawn(command)
  puts "Running command: #{command} (PID: #{pid})"

  connected = false
  while !connected do
    begin
      TCPSocket.open('localhost', 50080) { connected = true }
    rescue Errno::ECONNREFUSED
      sleep 0.3
    end
  end

  yield pid
ensure
  Process.kill('TERM', pid)
end

def run_tests(provisioner_mode:)
  file_dir = File.dirname(__FILE__)

  options = ["-p", provisioner_mode, "-s", "http", "-w", "#{file_dir}/test_config.json"]
  run_with_derrick(options) do |pid|
    puts "Running tests..."

    # Test the API
    response = request(:get, '/workspaces')
    raise "Expected empty workspaces, got #{response.inspect}" unless response["workspaces"] == []

    response = request(:post, '/workspaces', { 'env' => {} })
    raise "Expected workspace ID, got #{response.inspect}" unless response['id']

    id = response['id']

    puts "Test that we can get the workspace"
    response = request(:get, '/workspaces')
    raise "Expected empty workspaces, got #{response.inspect}" unless response.dig("workspaces", 0, "id") == id

    puts "Test that we can run a command"
    response = request(:post, "/workspaces/#{id}/cmd_with_output", { 'cmd' => 'echo hello' })
    raise "Expected output, got #{response.inspect}" unless response["output"] == "hello\n" && response["exit_code"] == 0

    puts "Test that we can list files in the workspace"
    response = request(:post, "/workspaces/#{id}/cmd_with_output", { 'cmd' => 'ls ./code/swiftide-ask' })
    raise "Expected output, got #{response.inspect}" unless response["output"].include?("Cargo.toml") && response["exit_code"] == 0

    puts "Test that we can run a command in the workspace"
    response = request(:post, "/workspaces/#{id}/cmd_with_output", { 'cmd' => 'cd ./code/swiftide-ask && ls' })
    raise "Expected output, got #{response.inspect}" unless response["output"].include?("Cargo.toml") && response["exit_code"] == 0

    puts "Test that we can run multiline commands"
    response = request(:post, "/workspaces/#{id}/cmd_with_output", { 'cmd' => "echo hello\necho world" })
    raise "Expected output, got #{response.inspect}" unless response["output"].include?("hello\nworld\n") && response["exit_code"] == 0

    puts "Test that the setup script ran successfully"
    response = request(:post, "/workspaces/#{id}/cmd_with_output", { 'cmd' => 'cat /tmp/hello.txt' })
    raise "Expected output, got #{response.inspect}" unless response["output"].include?("Hello World") && response["exit_code"] == 0

    puts "Test that we can set environment variables"
    response = request(:post, "/workspaces/#{id}/cmd_with_output", { 'cmd' => 'echo $HELLO', 'env' => { 'HELLO' => 'WORLD' } })
    raise "Expected output, got #{response.inspect}" unless response["output"].include?("WORLD") && response["exit_code"] == 0

    script_with_heredoc = <<~SCRIPT
      cat <<-"EOF" > /tmp/hello.txt
      hello
      world
      EOF
      cat /tmp/hello.txt
    SCRIPT

    puts "Test that we can use HEREDOCs"
    response = request(:post, "/workspaces/#{id}/cmd_with_output", { 'cmd' => script_with_heredoc })
    raise "Expected output, got #{response.inspect}" unless response["output"].include?("hello\nworld\n") && response["exit_code"] == 0

    puts "Test that failed commands return exit code 1"
    response = request(:post, "/workspaces/#{id}/cmd_with_output", { 'cmd' => 'ls /nonexistent/directory' })
    raise "Expected exit code 1, got #{response.inspect}" unless response["exit_code"] == 2 && response["output"].include?("No such file or directory")

    puts "Test that we can write a file"
    content = "Hello, world!"
    content_encoded = Base64.encode64(content)
    response = request(:post, "/workspaces/#{id}/write_file", { 'path' => '/tmp/test.txt', 'content' => content_encoded })
    raise "Expected no error, got #{response.inspect}" unless response["error"].nil?

    response = request(:post, "/workspaces/#{id}/read_file", { 'path' => '/tmp/test.txt' })
    raise "Expected output, got #{response.inspect}" unless response == content
  end
end

["docker"].each do |provisioner_mode|
  puts "Running tests in #{provisioner_mode} mode..."
  run_tests(provisioner_mode: provisioner_mode)
end

puts "\n\nAll tests passed!\n\n"