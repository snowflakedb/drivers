package net.snowflake.client.internal.codegen;

import java.io.IOException;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;
import javax.annotation.processing.AbstractProcessor;
import javax.annotation.processing.RoundEnvironment;
import javax.annotation.processing.SupportedAnnotationTypes;
import javax.annotation.processing.SupportedSourceVersion;
import javax.lang.model.SourceVersion;
import javax.lang.model.element.Element;
import javax.lang.model.element.ExecutableElement;
import javax.lang.model.element.TypeElement;
import javax.lang.model.element.VariableElement;
import javax.lang.model.type.DeclaredType;
import javax.lang.model.type.TypeKind;
import javax.lang.model.type.TypeMirror;
import javax.lang.model.util.ElementFilter;
import javax.tools.Diagnostic;
import javax.tools.JavaFileObject;

/**
 * Generates decorator classes for {@link JdbcBoundary}-annotated impls. Reads the interfaces
 * directly from the class's {@code implements} clause, skipping internal ones.
 */
@SupportedAnnotationTypes("net.snowflake.client.internal.codegen.JdbcBoundary")
@SupportedSourceVersion(SourceVersion.RELEASE_8)
public class DecoratorProcessor extends AbstractProcessor {

  private static final Set<String> SKIPPED_INTERFACES = new HashSet<>();
  private static final Set<String> SKIPPED_METHODS = new HashSet<>();

  static {
    SKIPPED_INTERFACES.add("net.snowflake.client.internal.util.DelegatingWrapper");
    SKIPPED_INTERFACES.add("java.sql.Wrapper");
    SKIPPED_INTERFACES.add("java.io.Serializable");
    SKIPPED_INTERFACES.add("java.lang.AutoCloseable");
    SKIPPED_INTERFACES.add("java.io.Closeable");

    SKIPPED_METHODS.add("unwrap");
    SKIPPED_METHODS.add("isWrapperFor");
  }

  private static final String DECORATOR_PACKAGE = "net.snowflake.client.internal.api.decorator";

  @Override
  public boolean process(Set<? extends TypeElement> annotations, RoundEnvironment roundEnv) {
    for (Element element : roundEnv.getElementsAnnotatedWith(JdbcBoundary.class)) {
      if (!(element instanceof TypeElement)) {
        continue;
      }
      try {
        generateDecorator((TypeElement) element);
      } catch (IOException e) {
        processingEnv
            .getMessager()
            .printMessage(
                Diagnostic.Kind.ERROR,
                "Failed to generate decorator: " + e.getMessage(),
                element);
      }
    }
    return true;
  }

  private void generateDecorator(TypeElement implElement) throws IOException {
    String implQualifiedName = implElement.getQualifiedName().toString();
    String implSimpleName = implElement.getSimpleName().toString();
    String implPackage = implQualifiedName.substring(0, implQualifiedName.lastIndexOf('.'));
    String decoratorName = "Decorated" + implSimpleName;

    // Read interfaces from the implements clause, filtering out internal ones
    List<TypeElement> interfaceElements = new ArrayList<>();
    for (TypeMirror iface : implElement.getInterfaces()) {
      if (iface.getKind() != TypeKind.DECLARED) {
        continue;
      }
      TypeElement ifaceElement = (TypeElement) ((DeclaredType) iface).asElement();
      if (!SKIPPED_INTERFACES.contains(ifaceElement.getQualifiedName().toString())) {
        interfaceElements.add(ifaceElement);
      }
    }

    if (interfaceElements.isEmpty()) {
      processingEnv
          .getMessager()
          .printMessage(
              Diagnostic.Kind.WARNING,
              "@JdbcBoundary: no delegatable interfaces found on " + implSimpleName,
              implElement);
      return;
    }

    // Build index of impl methods for parameter name resolution
    Map<String, ExecutableElement> implMethods = new LinkedHashMap<>();
    for (ExecutableElement m : ElementFilter.methodsIn(implElement.getEnclosedElements())) {
      implMethods.put(methodSignature(m), m);
    }

    // Collect all abstract methods from the interfaces
    Map<String, MethodInfo> methods = new LinkedHashMap<>();
    for (TypeElement iface : interfaceElements) {
      collectMethods(iface, methods);
    }

    // Resolve parameter names from impl where possible
    for (Map.Entry<String, MethodInfo> entry : methods.entrySet()) {
      ExecutableElement implMethod = implMethods.get(entry.getKey());
      if (implMethod != null) {
        entry.getValue().implMethod = implMethod;
      }
    }

    // Collect imports
    TreeSet<String> imports = new TreeSet<>();
    imports.add("java.sql.SQLException");
    imports.add("javax.annotation.Generated");
    imports.add(DECORATOR_PACKAGE + ".AbstractDecorator");
    imports.add(DECORATOR_PACKAGE + ".Telemetry");
    for (TypeElement iface : interfaceElements) {
      String ifaceName = iface.getQualifiedName().toString();
      if (!ifaceName.startsWith("java.lang.")) {
        imports.add(ifaceName);
      }
    }
    for (MethodInfo mi : methods.values()) {
      addImports(imports, mi);
    }

    // Generate source in the same package as the impl
    String qualifiedDecoratorName = implPackage + "." + decoratorName;
    JavaFileObject sourceFile =
        processingEnv.getFiler().createSourceFile(qualifiedDecoratorName, implElement);

    try (PrintWriter w = new PrintWriter(sourceFile.openWriter())) {
      w.println("package " + implPackage + ";");
      w.println();

      for (String imp : imports) {
        // Skip same-package imports
        if (imp.startsWith(implPackage + ".")
            && !imp.substring(implPackage.length() + 1).contains(".")) {
          continue;
        }
        w.println("import " + imp + ";");
      }
      w.println();

      w.println("@Generated(\"" + DecoratorProcessor.class.getName() + "\")");
      w.print("public final class " + decoratorName);
      w.println();
      w.print("    extends AbstractDecorator<" + implSimpleName + ">");
      w.println();
      w.print("    implements ");
      for (int i = 0; i < interfaceElements.size(); i++) {
        if (i > 0) {
          w.print(", ");
        }
        w.print(interfaceElements.get(i).getSimpleName());
      }
      w.println(" {");
      w.println();

      w.println(
          "  public "
              + decoratorName
              + "("
              + implSimpleName
              + " delegate, Telemetry telemetry) {");
      w.println("    super(delegate, telemetry);");
      w.println("  }");

      boolean needsSneakyThrow = false;
      for (MethodInfo mi : methods.values()) {
        if (delegateThrowsChecked(mi)) {
          needsSneakyThrow = true;
          break;
        }
      }

      for (MethodInfo mi : methods.values()) {
        w.println();
        writeMethod(w, mi);
      }

      if (needsSneakyThrow) {
        w.println();
        w.println("  @SuppressWarnings(\"unchecked\")");
        w.println("  private static <E extends Throwable>"
            + " RuntimeException sneakyThrow(Throwable t) throws E {");
        w.println("    throw (E) t;");
        w.println("  }");
      }

      w.println("}");
    }
  }

  private void collectMethods(TypeElement iface, Map<String, MethodInfo> methods) {
    for (ExecutableElement method : ElementFilter.methodsIn(iface.getEnclosedElements())) {
      String name = method.getSimpleName().toString();
      if (SKIPPED_METHODS.contains(name)) {
        continue;
      }
      String sig = methodSignature(method);
      if (!methods.containsKey(sig)) {
        methods.put(sig, new MethodInfo(method));
      }
    }
    for (TypeMirror superIface : iface.getInterfaces()) {
      if (superIface.getKind() == TypeKind.DECLARED) {
        TypeElement superElement = (TypeElement) ((DeclaredType) superIface).asElement();
        String superName = superElement.getQualifiedName().toString();
        if (!SKIPPED_INTERFACES.contains(superName)) {
          collectMethods(superElement, methods);
        }
      }
    }
  }

  private String methodSignature(ExecutableElement method) {
    StringBuilder sb = new StringBuilder();
    sb.append(method.getSimpleName());
    sb.append('(');
    for (int i = 0; i < method.getParameters().size(); i++) {
      if (i > 0) {
        sb.append(',');
      }
      sb.append(
          processingEnv
              .getTypeUtils()
              .erasure(method.getParameters().get(i).asType())
              .toString());
    }
    sb.append(')');
    return sb.toString();
  }

  private void addImports(TreeSet<String> imports, MethodInfo mi) {
    addTypeImport(imports, mi.method.getReturnType());
    for (VariableElement param : mi.method.getParameters()) {
      addTypeImport(imports, param.asType());
    }
    for (TypeMirror thrown : mi.method.getThrownTypes()) {
      addTypeImport(imports, thrown);
    }
  }

  private void addTypeImport(TreeSet<String> imports, TypeMirror type) {
    if (type.getKind() == TypeKind.DECLARED) {
      DeclaredType dt = (DeclaredType) type;
      TypeElement te = (TypeElement) dt.asElement();
      String qualified = te.getQualifiedName().toString();
      if (!qualified.startsWith("java.lang.")) {
        imports.add(qualified);
      }
      for (TypeMirror arg : dt.getTypeArguments()) {
        addTypeImport(imports, arg);
      }
    }
  }

  private void writeMethod(PrintWriter w, MethodInfo mi) {
    ExecutableElement method = mi.method;
    String returnType = typeToString(method.getReturnType());
    String methodName = method.getSimpleName().toString();
    boolean isVoid = method.getReturnType().getKind() == TypeKind.VOID;
    boolean noTelemetry =
        mi.implMethod != null && mi.implMethod.getAnnotation(NoTelemetry.class) != null;
    boolean needsTryCatch = delegateThrowsChecked(mi);

    ExecutableElement nameSource = mi.implMethod != null ? mi.implMethod : method;

    w.println("  @Override");
    w.print("  public " + returnType + " " + methodName + "(");

    List<? extends VariableElement> params = method.getParameters();
    List<? extends VariableElement> nameParams = nameSource.getParameters();
    for (int i = 0; i < params.size(); i++) {
      if (i > 0) {
        w.print(", ");
      }
      w.print(typeToString(params.get(i).asType()) + " " + nameParams.get(i).getSimpleName());
    }
    w.print(") throws SQLException {");
    w.println();

    StringBuilder args = new StringBuilder();
    for (int i = 0; i < params.size(); i++) {
      if (i > 0) {
        args.append(", ");
      }
      args.append(nameParams.get(i).getSimpleName());
    }

    String delegateCall = "delegate." + methodName + "(" + args + ")";
    // Op-name uses the *declaring* JDBC interface, so inherited methods read as e.g.
    // "Statement.execute" even on the PreparedStatement decorator (Decision 2).
    String opName = declaringInterfaceName(method) + "." + methodName;
    String callPrefix = noTelemetry ? "" : "\"" + opName + "\", ";

    if (needsTryCatch) {
      writeTryCatchLambda(w, delegateCall, callPrefix, isVoid);
    } else {
      writeDirectLambda(w, delegateCall, callPrefix, isVoid);
    }

    w.println("  }");
  }

  private void writeDirectLambda(
      PrintWriter w, String delegateCall, String callPrefix, boolean isVoid) {
    if (isVoid) {
      w.println("    run(" + callPrefix + "() -> " + delegateCall + ");");
    } else {
      w.println("    return call(" + callPrefix + "() -> " + delegateCall + ");");
    }
  }

  private void writeTryCatchLambda(
      PrintWriter w, String delegateCall, String callPrefix, boolean isVoid) {
    if (isVoid) {
      w.println("    run(" + callPrefix + "() -> {");
      w.println("      try { " + delegateCall + "; }");
    } else {
      w.println("    return call(" + callPrefix + "() -> {");
      w.println("      try { return " + delegateCall + "; }");
    }
    w.println("      catch (SQLException e) { throw sneakyThrow(e); }");
    w.println("    });");
  }

  /**
   * Simple name of the interface that declares {@code method} — the collected interface method's
   * enclosing element is always the interface it was declared on, so inherited methods keep the
   * declaring interface (e.g. {@code Statement}) rather than the implementing one.
   */
  private String declaringInterfaceName(ExecutableElement method) {
    return method.getEnclosingElement().getSimpleName().toString();
  }

  private boolean delegateThrowsChecked(MethodInfo mi) {
    ExecutableElement source = mi.implMethod != null ? mi.implMethod : mi.method;
    for (TypeMirror thrown : source.getThrownTypes()) {
      if (thrown.getKind() != TypeKind.DECLARED) {
        continue;
      }
      TypeElement thrownElement = (TypeElement) ((DeclaredType) thrown).asElement();
      String name = thrownElement.getQualifiedName().toString();
      if (!name.equals("java.lang.RuntimeException")
          && !isSubtypeOf(thrownElement, "java.lang.RuntimeException")
          && !name.equals("java.lang.Error")
          && !isSubtypeOf(thrownElement, "java.lang.Error")) {
        return true;
      }
    }
    return false;
  }

  private boolean isSubtypeOf(TypeElement element, String superTypeName) {
    TypeElement superType = processingEnv.getElementUtils().getTypeElement(superTypeName);
    return superType != null
        && processingEnv.getTypeUtils().isSubtype(element.asType(), superType.asType());
  }

  private String typeToString(TypeMirror type) {
    if (type.getKind() == TypeKind.DECLARED) {
      DeclaredType dt = (DeclaredType) type;
      TypeElement te = (TypeElement) dt.asElement();
      String simpleName = te.getSimpleName().toString();
      List<? extends TypeMirror> typeArgs = dt.getTypeArguments();
      if (typeArgs.isEmpty()) {
        return simpleName;
      }
      StringBuilder sb = new StringBuilder(simpleName);
      sb.append('<');
      for (int i = 0; i < typeArgs.size(); i++) {
        if (i > 0) {
          sb.append(", ");
        }
        sb.append(typeToString(typeArgs.get(i)));
      }
      sb.append('>');
      return sb.toString();
    }
    return type.toString();
  }

  private static class MethodInfo {
    final ExecutableElement method;
    ExecutableElement implMethod;

    MethodInfo(ExecutableElement method) {
      this.method = method;
    }
  }
}
