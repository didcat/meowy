#include <llvm/AsmParser/Parser.h>
#include <llvm/IR/LegacyPassManager.h>
#include <llvm/IR/LLVMContext.h>
#include <llvm/IR/Module.h>
#include <llvm/IR/Verifier.h>
#include <llvm/MC/TargetRegistry.h>
#include <llvm/Passes/PassBuilder.h>
#include <llvm/Support/FileSystem.h>
#include <llvm/Support/MemoryBuffer.h>
#include <llvm/Support/SourceMgr.h>
#include <llvm/Support/TargetSelect.h>
#include <llvm/Support/raw_ostream.h>
#include <llvm/Target/TargetMachine.h>
#include <algorithm>
#include <cstddef>
#include <cstring>
#include <memory>
#include <mutex>
#include <string>

static int fail(const std::string &message, char *error, std::size_t capacity) {
    if (capacity != 0) {
        const std::size_t size = std::min(message.size(), capacity - 1);
        std::memcpy(error, message.data(), size);
        error[size] = '\0';
    }
    return 1;
}

extern "C" int meowy_emit_object_v1(const char *ir, std::size_t size, const char *path,
                                    std::size_t path_size, int release, char *error,
                                    std::size_t capacity) {
    static std::once_flag initialized;
    std::call_once(initialized, [] {
        LLVMInitializeX86TargetInfo();
        LLVMInitializeX86Target();
        LLVMInitializeX86TargetMC();
        LLVMInitializeX86AsmPrinter();
        LLVMInitializeX86AsmParser();
    });
    llvm::LLVMContext context;
    llvm::SMDiagnostic diagnostic;
    const std::unique_ptr<llvm::MemoryBuffer> buffer = llvm::MemoryBuffer::getMemBufferCopy(llvm::StringRef(ir, size), "<meowy>");
    std::unique_ptr<llvm::Module> module = llvm::parseAssembly(buffer->getMemBufferRef(), diagnostic, context);
    if (!module) {
        std::string message;
        llvm::raw_string_ostream stream(message);
        diagnostic.print("LLVM IR parser", stream);
        return fail(message, error, capacity);
    }
    std::string message;
    llvm::raw_string_ostream stream(message);
    if (llvm::verifyModule(*module, &stream)) {
        return fail("LLVM verification: " + message, error, capacity);
    }
    const llvm::Triple triple("x86_64-unknown-linux-gnu");
    const llvm::Target *target = llvm::TargetRegistry::lookupTarget(triple, message);
    if (target == nullptr) {
        return fail("LLVM target: " + message, error, capacity);
    }
    llvm::TargetOptions options;
    options.FunctionSections = true;
    options.DataSections = true;
    std::unique_ptr<llvm::TargetMachine> machine(target->createTargetMachine(
        triple, "x86-64", "+sse2", options, llvm::Reloc::PIC_, std::nullopt,
        release ? llvm::CodeGenOptLevel::Aggressive : llvm::CodeGenOptLevel::None));
    if (!machine) {
        return fail("LLVM target machine construction failed", error, capacity);
    }
    module->setTargetTriple(triple);
    module->setDataLayout(machine->createDataLayout());
    llvm::PassBuilder builder(machine.get());
    llvm::LoopAnalysisManager loops;
    llvm::FunctionAnalysisManager functions;
    llvm::CGSCCAnalysisManager calls;
    llvm::ModuleAnalysisManager modules;
    builder.registerLoopAnalyses(loops);
    builder.registerFunctionAnalyses(functions);
    builder.registerCGSCCAnalyses(calls);
    builder.registerModuleAnalyses(modules);
    builder.crossRegisterProxies(loops, functions, calls, modules);
    llvm::ModulePassManager pipeline = release
        ? builder.buildPerModuleDefaultPipeline(llvm::OptimizationLevel::O2)
        : builder.buildO0DefaultPipeline(llvm::OptimizationLevel::O0);
    pipeline.run(*module, modules);
    message.clear();
    if (llvm::verifyModule(*module, &stream)) {
        return fail("LLVM verification after optimization: " + message, error, capacity);
    }
    std::error_code code;
    llvm::raw_fd_ostream file(llvm::StringRef(path, path_size), code, llvm::sys::fs::OF_None);
    if (code) {
        return fail("object output: " + code.message(), error, capacity);
    }
    llvm::legacy::PassManager passes;
    if (machine->addPassesToEmitFile(passes, file, nullptr, llvm::CodeGenFileType::ObjectFile, false)) {
        return fail("LLVM target cannot emit object files", error, capacity);
    }
    passes.run(*module);
    file.flush();
    if (file.has_error()) {
        const std::string failure = file.error().message();
        file.clear_error();
        return fail("object output: " + failure, error, capacity);
    }
    return 0;
}
